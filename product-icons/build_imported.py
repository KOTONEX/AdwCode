#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""从固定版本 SVG 生成独立授权的字体；再生成时依赖 fontTools，描边另需 skia-pathops。"""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from xml.etree import ElementTree

from fontTools.fontBuilder import FontBuilder
from fontTools.misc.transform import Identity, Transform
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.svgLib.path import parse_path
from fontTools.svgLib.path.shapes import PathBuilder

ROOT = Path(__file__).resolve().parent


def svg_transform(value: str) -> Transform:
    """只接受已选资产使用的变换；新格式必须明确支持，不能静默忽略。"""
    result = Identity
    remainder = value
    for match in re.finditer(r"(matrix|translate|scale)\s*\(([^)]+)\)", value):
        values = tuple(float(v) for v in re.split(r"[\s,]+", match[2].strip()))
        if match[1] == "matrix" and len(values) == 6:
            current = Transform(*values)
        elif match[1] == "translate" and len(values) in (1, 2):
            current = Identity.translate(values[0], values[1] if len(values) == 2 else 0)
        elif match[1] == "scale" and len(values) in (1, 2):
            current = Identity.scale(values[0], values[1] if len(values) == 2 else values[0])
        else:
            raise ValueError(f"不支持的 SVG 变换：{match[0]}")
        result = result.transform(current)
        remainder = remainder.replace(match[0], "", 1)
    if remainder.strip():
        raise ValueError(f"不支持的 SVG 变换：{value}")
    return result


def draw_svg(source: Path, pen: TTGlyphPen) -> None:
    """读取可见轮廓，忽略定义和完全在画布外的导出残留；拒绝可见复杂特效。"""
    root = ElementTree.parse(source).getroot()
    view = tuple(float(v) for v in root.attrib.get("viewBox", "0 0 16 16").split())
    if view != (0, 0, 16, 16):
        raise ValueError(f"{source.name} 需要 16 × 16 画布")
    target = TransformPen(Cu2QuPen(pen, 1, reverse_direction=True), (64, 0, 0, -64, 0, 896))

    def visit(element: ElementTree.Element, transform: Transform, effects: bool = False) -> None:
        # 编辑器元数据不参与绘制；SVG 命名空间内的未知元素仍应报错。
        if element.tag.startswith("{") and not element.tag.startswith("{http://www.w3.org/2000/svg}"):
            return
        tag = element.tag.rsplit("}", 1)[-1]
        if tag in {"defs", "filter", "mask", "clipPath", "metadata", "title", "desc"}:
            return
        transform = transform.transform(svg_transform(element.attrib.get("transform", "")))
        effects = effects or any(k in element.attrib for k in ("mask", "clip-path", "filter"))
        if tag in {"svg", "g"}:
            for child in element:
                visit(child, transform, effects)
            return
        paths = PathBuilder()
        if not paths.add_path_from_element(element):
            raise ValueError(f"{source.name} 存在不支持的元素：{tag}")
        style = dict(part.split(":", 1) for part in element.attrib.get("style", "").split(";") if ":" in part)
        attributes = {**element.attrib, **{key.strip(): value.strip() for key, value in style.items()}}
        stroked = attributes.get("stroke", "none") != "none"
        if attributes.get("fill") == "none" and not stroked:
            return
        if stroked and attributes.get("fill") != "none":
            raise ValueError(f"{source.name} 同时包含填充与描边，需要单独转换")
        for path in paths.paths:
            recording = RecordingPen()
            if stroked:
                # 官方新 SVG 的描边需展开为轮廓，保留宽度、端点与连接形状。
                try:
                    import pathops
                except ImportError as error:
                    raise RuntimeError("再生成描边图标还需要 skia-pathops；扩展运行不需要该依赖") from error
                if "stroke-dasharray" in attributes or "stroke-dashoffset" in attributes:
                    raise ValueError(f"{source.name} 使用尚未支持的虚线描边")
                outline = pathops.Path()
                parse_path(path, outline.getPen())
                caps = {"butt": pathops.LineCap.BUTT_CAP, "round": pathops.LineCap.ROUND_CAP, "square": pathops.LineCap.SQUARE_CAP}
                joins = {"miter": pathops.LineJoin.MITER_JOIN, "round": pathops.LineJoin.ROUND_JOIN, "bevel": pathops.LineJoin.BEVEL_JOIN}
                outline.stroke(float(attributes.get("stroke-width", "1")),
                               caps[attributes.get("stroke-linecap", "butt")],
                               joins[attributes.get("stroke-linejoin", "miter")],
                               float(attributes.get("stroke-miterlimit", "4")))
                outline.convertConicsToQuads(0.01)
                outline.simplify()
                outline.draw(TransformPen(recording, transform))
                # SVG 的画布会裁掉描边的微小越界，字体也保留同样的可见区域。
                visible = pathops.Path()
                recording.replay(visible.getPen())
                viewport = pathops.Path()
                parse_path("M0 0H16V16H0Z", viewport.getPen())
                visible = pathops.op(visible, viewport, pathops.PathOp.INTERSECTION)
                recording = RecordingPen()
                visible.draw(recording)
            else:
                parse_path(path, TransformPen(recording, transform))
            # 同一 path 也可能包含画布外的独立导出残留，按闭合轮廓检查。
            contours = [RecordingPen()]
            for operation, args in recording.value:
                if operation == "moveTo" and contours[-1].value:
                    contours.append(RecordingPen())
                contours[-1].value.append((operation, args))
            for contour in contours:
                bounds = BoundsPen(None)
                contour.replay(bounds)
                if bounds.bounds is None:
                    continue
                left, top, right, bottom = bounds.bounds
                if right <= 0 or bottom <= 0 or left >= 16 or top >= 16:
                    continue
                if effects:
                    raise ValueError(f"{source.name} 存在可见遮罩或裁切，不能直接转换为轮廓字体")
                # 上游少量导出轮廓在边缘有不足 1/16 像素的舍入偏差。
                if min(left, top) < -0.0625 or max(right, bottom) > 16.0625:
                    raise ValueError(f"{source.name} 的可见轮廓超出画布")
                contour.replay(target)

    visit(root, Identity)


def main() -> None:
    sources = json.loads((ROOT / "sources.json").read_text(encoding="utf-8"))
    for font in sources["fonts"]:
        entries = font["glyphs"]
        builder = FontBuilder(1024, isTTF=True)
        order = [".notdef"] + ["uni" + g["codepoint"].upper() for g in entries]
        builder.setupGlyphOrder(order)
        glyphs = {".notdef": TTGlyphPen(None).glyph()}
        for entry, name in zip(entries, order[1:]):
            source = ROOT / entry["file"]
            if hashlib.sha256(source.read_bytes()).hexdigest() != entry["sha256"]:
                raise ValueError(f"来源文件已变化：{source.name}；需同步来源记录")
            pen = TTGlyphPen(None)
            draw_svg(source, pen)
            glyph = pen.glyph()
            if glyph.numberOfContours == 0:
                raise ValueError(f"字形为空：{source.name}")
            glyphs[name] = glyph
        builder.setupGlyf(glyphs)
        for glyph in glyphs.values():
            glyph.recalcBounds(glyphs)
        builder.setupHorizontalMetrics({name: (1024, getattr(glyphs[name], "xMin", 0)) for name in order})
        builder.setupHorizontalHeader(ascent=896, descent=-128)
        builder.setupCharacterMap({int(g["codepoint"], 16): name for g, name in zip(entries, order[1:])})
        ps_name = font["family"].replace(" ", "") + "-Regular"
        builder.setupNameTable({
            "familyName": font["family"], "styleName": "Regular", "fullName": font["family"],
            "uniqueFontIdentifier": ps_name, "psName": ps_name, "version": "Version 1.0",
            "copyright": font["attribution"],
            "licenseDescription": font["license"] + "；由 AdwCode 转换为单色轮廓；来源见 sources.json",
            "licenseInfoURL": font["license_url"],
        })
        builder.setupOS2(sTypoAscender=896, sTypoDescender=-128, usWinAscent=896, usWinDescent=128)
        builder.setupPost()
        builder.font["head"].created = builder.font["head"].modified = 3863548800
        builder.font.recalcTimestamp = False
        builder.save(ROOT / font["output"])
        state = "启用" if font.get("enabled", True) else "备用"
        print(f"已生成 {font['output']}：{len(entries)} 个字形，{font['license']}，{state}")


if __name__ == "__main__":
    main()
