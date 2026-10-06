#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode 贡献者
"""用 fontTools 将自有 SVG 字形生成为单色产品图标字体，仅再生成时需要依赖。"""

from pathlib import Path
from xml.etree import ElementTree

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.svgLib.path import parse_path

ROOT = Path(__file__).resolve().parent
sources = sorted((ROOT / "符号图标").glob("*.svg"))
builder = FontBuilder(1024, isTTF=True)
order = [".notdef"] + ["uni" + source.stem.upper() for source in sources]
builder.setupGlyphOrder(order)
glyphs = {".notdef": TTGlyphPen(None).glyph()}
for source, name in zip(sources, order[1:]):
    pen = TTGlyphPen(None)
    transformed = TransformPen(Cu2QuPen(pen, 1, reverse_direction=True), (64, 0, 0, -64, 0, 896))
    for element in ElementTree.parse(source).iter("{http://www.w3.org/2000/svg}path"):
        parse_path(element.attrib["d"], transformed)
    glyphs[name] = pen.glyph()
builder.setupGlyf(glyphs)
for glyph in glyphs.values():
    glyph.recalcBounds(glyphs)
builder.setupHorizontalMetrics({name: (1024, getattr(glyphs[name], "xMin", 0)) for name in order})
builder.setupHorizontalHeader(ascent=896, descent=-128)
builder.setupCharacterMap({int(source.stem, 16): name for source, name in zip(sources, order[1:])})
builder.setupNameTable(
    {
        "familyName": "AdwCode Symbols",
        "styleName": "Regular",
        "uniqueFontIdentifier": "AdwCodeSymbols-Regular",
        "fullName": "AdwCode Symbols",
        "psName": "AdwCodeSymbols-Regular",
        "version": "Version 1.0",
        "copyright": "2026 AdwCode 贡献者",
        "licenseDescription": "AGPL-3.0-or-later OR CC-BY-SA-4.0+；许可声明见 产品图标/LICENSE",
        "licenseInfoURL": "https://github.com/KOTONEX/AdwCode/blob/main/产品图标/LICENSE",
    }
)
builder.setupOS2(sTypoAscender=896, sTypoDescender=-128, usWinAscent=896, usWinDescent=128)
builder.setupPost()
# 固定时间戳，使同一份源资产生成的字体可复现。
builder.font["head"].created = builder.font["head"].modified = 3863548800
builder.font.recalcTimestamp = False
builder.save(ROOT / "adwcode-符号.ttf")
print(f"已生成 {len(sources)} 个单色字形")
