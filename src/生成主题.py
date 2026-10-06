#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""生成 AdwCode 主题并同步 package.json。

用法：
    python3.14t src/生成主题.py                     # blue + 当前系统强调色
    python3.14t src/生成主题.py --强调色 全部       # 全部九种 GNOME 强调色
    python3.14t src/生成主题.py --强调色 blue,teal # 指定强调色列表
    python3.14t src/生成主题.py --不读取系统         # 仅 blue
    python3.14t src/生成主题.py --校验             # 校验已生成的主题
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Optional, TypedDict, cast

sys.path.insert(0, str(Path(__file__).parent))

import 界面映射
import 语法映射
from 调色板 import ACCENT_LABELS, ACCENT_NAMES, MODE_LABELS, 解析颜色, 调色板对象

ROOT: Path = Path(__file__).parent.parent
THEMES: Path = ROOT / "themes"
DEFAULTS: Path = Path(__file__).parent / "vscode_defaults"


class 主题构建请求(TypedDict):
    """``构建计划()`` 生成的一项主题构建请求。"""

    mode: str
    accent: str
    variant: str
    hc: bool


class 主题注册项(TypedDict):
    """``package.json`` 中 ``contributes.themes`` 的一个条目。"""

    label: str
    uiTheme: str
    path: str


class 监视主题注册项(主题注册项, total=False):
    """``--watch`` 模式下附加的调试标记。"""

    _watch: bool


#: 生成的主题 JSON；``$schema`` 无法作为类语法字段，因此使用函数式写法。
#: 函数式写法的值是运行时求值的，故用 ``Optional`` 保持 Python 3.9 兼容。
主题对象 = TypedDict(
    "主题对象",
    {
        "$schema": str,
        "name": str,
        "type": str,
        "semanticHighlighting": bool,
        "colors": dict[str, str],
        "tokenColors": list[语法映射.语法规则],
        "semanticTokenColors": dict[str, Optional[str]],
    },
)


def 主题文件名(mode: str, accent: str, variant: str, high_contrast: bool = False) -> str:
    """主题文件名使用中文，保留项目的 adwcode 前缀。"""
    parts = ["adwcode"]
    if accent != "blue":
        parts.append(ACCENT_LABELS[accent])
    parts.append(MODE_LABELS[mode])
    if high_contrast:
        parts.append("高对比度")
    elif variant == "colorful":
        parts.append("彩色状态栏")
    elif variant == "default":
        parts.append("默认语法高亮")
    elif variant == "default-colorful":
        parts.append("默认语法高亮-彩色状态栏")
    return "-".join(parts) + ".json"


def 系统强调色() -> str | None:
    """读取 GNOME 强调色偏好。"""
    try:
        out = subprocess.run(
            ["gsettings", "get", "org.gnome.desktop.interface", "accent-color"],
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    value = out.stdout.strip().strip("'\"")
    return value if value in ACCENT_NAMES else None


def 解析强调色(spec: str | None, use_system: bool) -> list[str]:
    if spec == "全部":
        return list(ACCENT_NAMES)
    if spec == "系统":
        spec = None
        use_system = True
    if spec:
        wanted = [item.strip() for item in spec.split(",") if item.strip()]
        unknown = [item for item in wanted if item not in ACCENT_NAMES]
        if unknown:
            raise SystemExit(f"未知强调色：{', '.join(unknown)}")
        if not wanted:
            raise SystemExit("请至少指定一种强调色")
        return list(dict.fromkeys(wanted))
    accents = ["blue"]
    if use_system:
        accent = 系统强调色()
        if accent and accent not in accents:
            accents.append(accent)
    return accents


def 相对亮度(color: str) -> float:
    r, g, b, _ = 解析颜色(color)

    def channel(value: float) -> float:
        value /= 255
        return value / 12.92 if value <= 0.03928 else cast(float, ((value + 0.055) / 1.055) ** 2.4)

    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)


def 对比度(color_a: str, color_b: str) -> float:
    lum_a, lum_b = 相对亮度(color_a), 相对亮度(color_b)
    lighter, darker = max(lum_a, lum_b), min(lum_a, lum_b)
    return (lighter + 0.05) / (darker + 0.05)


def 主题标签(mode: str, accent: str) -> str:
    kind = MODE_LABELS[mode]
    if accent == "blue":
        return f"AdwCode {kind}"
    return f"AdwCode {ACCENT_LABELS[accent]} {kind}"


def 生成主题对象(mode: str, accent: str, variant: str, high_contrast: bool = False) -> 主题对象:
    """variant 取值：builder、colorful、default、default-colorful。"""
    scheme = 语法映射.编辑器颜色(mode)
    调色板 = 调色板对象(mode, accent=accent, high_contrast=high_contrast, scheme=scheme)
    colorful = variant in ("colorful", "default-colorful")
    default_syntax = variant in ("default", "default-colorful")

    if high_contrast:
        label = f"AdwCode {MODE_LABELS[mode]} 高对比度"
    else:
        label = 主题标签(mode, accent)
        if default_syntax:
            label += " · 默认语法高亮"
        if colorful:
            label += " · 彩色状态栏"

    语法颜色: list[语法映射.语法规则]
    if default_syntax:
        语法颜色 = cast(
            list[语法映射.语法规则],
            json.loads((DEFAULTS / f"{mode}.json").read_text(encoding="utf-8"))["tokenColors"],
        )
    else:
        语法颜色 = 语法映射.语法颜色(mode)

    return {
        "$schema": "vscode://schemas/color-theme",
        "name": label,
        "type": mode,
        "semanticHighlighting": True,
        "colors": 界面映射.生成界面颜色(调色板, colorful_status_bar=colorful),
        "tokenColors": 语法颜色,
        "semanticTokenColors": 语法映射.语义标记颜色(mode),
    }


def 构建计划(accents: list[str]) -> list[主题构建请求]:
    plan: list[主题构建请求] = []
    for mode in ("dark", "light"):
        for accent in accents:
            variants = (
                ["builder", "colorful", "default", "default-colorful"]
                if accent == "blue"
                else ["builder"]
            )
            for variant in variants:
                plan.append({"mode": mode, "accent": accent, "variant": variant, "hc": False})
        plan.append({"mode": mode, "accent": "blue", "variant": "builder", "hc": True})
    return plan


def 写入主题(plan: list[主题构建请求], watch: bool = False) -> list[主题注册项]:
    THEMES.mkdir(exist_ok=True)
    # 先生成所有内容，生成器失败时保留原有主题文件。
    prepared = [
        (item, 生成主题对象(item["mode"], item["accent"], item["variant"], item["hc"]))
        for item in plan
    ]
    written: set[Path] = set()
    entries: list[主题注册项] = []
    for item, theme in prepared:
        path = THEMES / 主题文件名(item["mode"], item["accent"], item["variant"], item["hc"])
        path.write_text(json.dumps(theme, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        written.add(path)
        if item["hc"]:
            ui_theme = "hc-black" if item["mode"] == "dark" else "hc-light"
        else:
            ui_theme = "vs-dark" if item["mode"] == "dark" else "vs"
        entry: 监视主题注册项 = {
            "label": theme["name"],
            "uiTheme": ui_theme,
            "path": f"./themes/{path.name}",
        }
        if watch:
            entry["_watch"] = True
        entries.append(entry)
    for old in THEMES.glob("*.json"):
        if old not in written:
            old.unlink()
    return entries


def 更新扩展清单(entries: list[主题注册项]) -> None:
    path = ROOT / "package.json"
    manifest = json.loads(path.read_text(encoding="utf-8"))
    manifest["contributes"]["themes"] = entries
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def 加载颜色键表() -> tuple[set[str] | None, set[str] | None]:
    """返回（注册表键集合, 内置主题键集合）。"""
    registry: set[str] | None = None
    builtin: set[str] | None = None
    registry_path = DEFAULTS / "registry_keys.json"
    builtin_path = DEFAULTS / "builtin_keys.json"
    if registry_path.exists():
        registry = set(json.loads(registry_path.read_text(encoding="utf-8")))
    if builtin_path.exists():
        builtin = set(json.loads(builtin_path.read_text(encoding="utf-8")))
    return registry, builtin


#: 有效但未列入官方文档的键（由 VS Code 自身注册，或为已弃用别名）。
LEGACY_KEYS: set[str] = {
    "contrastActiveBorder",
    "editorIndentGuide.background",
    "editorIndentGuide.activeBackground",
    "editorHoverWidget.highlightForeground",
    "editorSuggestWidget.selectedBackground",
    "editorWidget.shadow",
    "editorWidget.errorBorder",
    "editorWidget.warningBorder",
    "editorWidget.infoBorder",
    "editorWidget.prominentBackground",
    "editorWidget.prominentForeground",
    "editorWidget.prominentBorder",
    "extensionButton.prominentBorder",
    "editorPlaceholder.foreground",
    "scm.providerBorder",
}


def 校验() -> int:
    failures: int = 0
    registry, builtin = 加载颜色键表()
    if registry is None or builtin is None:
        print("失败：缺少 VS Code 颜色键表，请运行 更新默认数据.py 刷新数据")
        return 1
    paths = sorted(THEMES.glob("*.json"))
    if not paths:
        print("失败：没有可校验的主题，请先构建主题")
        return 1
    manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    entries = cast(list[主题注册项], manifest.get("contributes", {}).get("themes", []))
    registered = {(ROOT / entry["path"]).resolve(): entry for entry in entries}
    if len(registered) != len(entries) or set(registered) != {path.resolve() for path in paths}:
        print("失败：主题文件与 package.json 注册列表不一致，存在缺失、重复或未注册文件")
        failures += 1

    def valid_color(value: object) -> bool:
        return (
            isinstance(value, str)
            and re.fullmatch(r"#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?", value) is not None
        )

    our_keys: set[str] = set()
    labels: list[str] = []
    for path in paths:
        theme = json.loads(path.read_text(encoding="utf-8"))
        entry = registered.get(path.resolve())
        if theme["type"] not in ("dark", "light"):
            print(f"失败 {path.name}：未知主题类型 {theme['type']}")
            failures += 1
        if entry and (
            entry["label"] != theme["name"]
            or entry["uiTheme"]
            not in (("vs-dark", "hc-black") if theme["type"] == "dark" else ("vs", "hc-light"))
        ):
            print(f"失败 {path.name}：主题标签或明暗类型与注册信息不一致")
            failures += 1
        labels.append(theme["name"])
        our_keys |= set(theme["colors"])

        for key, value in theme["colors"].items():
            if not valid_color(value):
                print(f"失败 {path.name}: {key} 必须使用六位或八位十六进制颜色")
                failures += 1
        for rule in theme["tokenColors"]:
            if not rule.get("scope") or not rule.get("settings"):
                print(f"失败 {path.name}: 语法规则不完整 {rule}")
                failures += 1
            for key in ("foreground", "background"):
                if key in rule.get("settings", {}) and not valid_color(rule["settings"][key]):
                    print(f"失败 {path.name}: 语法规则 {key} 必须使用六位或八位十六进制颜色")
                    failures += 1
        for name, value in theme.get("semanticTokenColors", {}).items():
            if not valid_color(value):
                print(f"失败 {path.name}: 语义标记 {name} 必须使用六位或八位十六进制颜色")
                failures += 1
    if len(labels) != len(set(labels)):
        print("失败 主题名称重复")
        failures += 1

    # 产品图标主题
    icons_path = ROOT / "product-icons" / "adwcode.json"
    icon_entries = manifest.get("contributes", {}).get("productIconThemes", [])
    if not any(
        entry.get("id") == "adwcode" and (ROOT / entry["path"]).resolve() == icons_path.resolve()
        for entry in icon_entries
    ):
        print("失败：package.json 未正确注册 AdwCode 产品图标主题")
        failures += 1
    if icons_path.exists():
        icons = json.loads(icons_path.read_text(encoding="utf-8"))
        fonts = icons.get("fonts", [])
        definitions = icons.get("iconDefinitions", {})
        if not fonts or not definitions:
            print("失败 product-icons/adwcode.json: 缺少 fonts 或 iconDefinitions")
            failures += 1
        for font in fonts:
            sources = font.get("src", [])
            if not sources:
                print(f"失败 product-icons：字体 {font.get('id')} 缺少来源文件")
                failures += 1
            for source in sources:
                target = (icons_path.parent / source["path"]).resolve()
                if not target.is_file():
                    print(f"失败 product-icons：缺少 {source['path']}")
                    failures += 1
        font_ids = {font["id"] for font in fonts}
        if len(font_ids) != len(fonts):
            print("失败 product-icons: 字体标识重复")
            failures += 1
        # 同一字形可服务于语义相同的多个产品图标；检查引用，允许有意复用。
        for icon, definition in definitions.items():
            character = definition.get("fontCharacter", "")
            if definition.get("fontId") not in font_ids:
                print(f"失败 product-icons: {icon} 引用了未知字体")
                failures += 1
            if (
                not re.fullmatch(r"\\[0-9a-fA-F]{4,6}", character)
                or int(character[1:], 16) > 0x10FFFF
            ):
                print(f"失败 product-icons: {icon} 的字形码点无效")
                failures += 1
        print(f"产品图标：{len(definitions)} 个图标映射，{len(fonts)} 个字体")
    else:
        print("失败：缺少 product-icons/adwcode.json")
        failures += 1

    # 对每个生成的主题做对比度检查（含强调色、变体、高对比度）。
    checks: list[tuple[str, str, float]] = [
        ("editor.foreground", "editor.background", 4.5),
        ("button.foreground", "button.background", 2.5),  # GNOME 黄色的对比度约为 2.8
        ("textLink.foreground", "editor.background", 3.0),
        ("gitDecoration.deletedResourceForeground", "editor.background", 3.0),
        ("descriptionForeground", "editor.background", 2.5),
    ]
    failed_themes: int = 0
    for path in paths:
        theme = json.loads(path.read_text(encoding="utf-8"))
        colors = theme["colors"]
        problems: list[str] = []
        for fg_key, bg_key, minimum in checks:
            if not valid_color(colors.get(fg_key)) or not valid_color(colors.get(bg_key)):
                problems.append(f"{fg_key}/{bg_key} 缺少有效颜色")
                continue
            ratio = 对比度(colors[fg_key], colors[bg_key])
            if ratio < minimum:
                problems.append(f"{fg_key}/{bg_key} {ratio:.2f} < {minimum}")
        status = "通过" if not problems else "失败"
        print(f"{status} {theme['name']}")
        for problem in problems:
            print(f"       {problem}")
            failures += 1
        if problems:
            failed_themes += 1
    print(f"对比度：{len(paths) - failed_themes} 个主题通过，{failed_themes} 个主题存在问题")

    allowed = registry | builtin | LEGACY_KEYS
    unknown = sorted(our_keys - allowed)
    missing = sorted(builtin - our_keys)
    print(
        f"\n颜色键：已定义 {len(our_keys)} 个 | 未覆盖内置键 {len(missing)} 个 | 未知键 {len(unknown)} 个"
    )
    if unknown:
        print("未知颜色键（可能存在拼写错误）：")
        for key in unknown:
            print(f"  {key}")
        failures += 1
    if missing:
        print("未覆盖的颜色键（VS Code 将使用默认主题）：")
        for key in missing[:60]:
            print(f"  {key}")
        if len(missing) > 60:
            print(f"  ……另有 {len(missing) - 60} 个")

    print(f"\n主题数量：{len(labels)}")
    return 1 if failures else 0


def 入口() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--强调色", dest="accents", help="使用 '全部'、'系统' 或以逗号分隔的强调色列表"
    )
    parser.add_argument(
        "--不读取系统", dest="no_system", action="store_true", help="不读取系统强调色"
    )
    parser.add_argument("--校验", dest="check", action="store_true", help="仅校验已生成的主题")
    parser.add_argument(
        "--监视",
        dest="watch",
        action="store_true",
        help="添加未公开的 _watch 标记，让 VS Code 在保存主题 JSON 时重新加载（仅用于开发）",
    )
    args = parser.parse_args()

    if args.check:
        return 校验()

    accents = 解析强调色(args.accents if args.accents != "system" else None, not args.no_system)
    print(f"强调色： {', '.join(accents)}")
    entries = 写入主题(构建计划(accents), watch=args.watch)
    if args.watch:
        print("监视模式：已为主题条目添加 _watch 标记")
    更新扩展清单(entries)
    for entry in entries:
        print(f"  {entry['uiTheme']:9} {entry['label']}")
    print(f"已将 {len(entries)} 个主题写入 {THEMES.relative_to(ROOT)}/")
    return 0


if __name__ == "__main__":
    raise SystemExit(入口())
