#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""生成 AdwCode 主题并同步 package.json。

用法：
    python3.14t src/build.py                     # blue + 当前系统强调色
    python3.14t src/build.py --accents all       # 全部九种 GNOME 强调色
    python3.14t src/build.py --accents blue,teal # 指定强调色列表
    python3.14t src/build.py --no-system         # 仅 blue
    python3.14t src/build.py --check             # 校验已生成的主题
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Optional, TypedDict, cast

sys.path.insert(0, str(Path(__file__).parent))

import mapping
import tokens
from palette import ACCENT_LABELS, ACCENT_NAMES, MODE_LABELS, Palette, parse_color

ROOT: Path = Path(__file__).parent.parent
THEMES: Path = ROOT / "themes"
DEFAULTS: Path = Path(__file__).parent / "vscode_defaults"


class ThemeRequest(TypedDict):
    """``build_plan()`` 生成的一项主题构建请求。"""

    mode: str
    accent: str
    variant: str
    hc: bool


class ThemeEntry(TypedDict):
    """``package.json`` 中 ``contributes.themes`` 的一个条目。"""

    label: str
    uiTheme: str
    path: str


class WatchThemeEntry(ThemeEntry, total=False):
    """``--watch`` 模式下附加的调试标记。"""

    _watch: bool


#: 生成的主题 JSON；``$schema`` 无法作为类语法字段，因此使用函数式写法。
#: 函数式写法的值是运行时求值的，故用 ``Optional`` 保持 Python 3.9 兼容。
Theme = TypedDict(
    "Theme",
    {
        "$schema": str,
        "name": str,
        "type": str,
        "semanticHighlighting": bool,
        "colors": dict[str, str],
        "tokenColors": list[tokens.TokenRule],
        "semanticTokenColors": dict[str, Optional[str]],
    },
)


def theme_filename(mode: str, accent: str, variant: str, high_contrast: bool = False) -> str:
    """主题文件名（保持 ASCII，与主题标签的中文解耦）。"""
    parts = ["adwaita"]
    if accent != "blue":
        parts.append(accent)
    parts.append(mode)
    if high_contrast:
        parts.append("high-contrast")
    elif variant == "colorful":
        parts.append("colorful-status-bar")
    elif variant == "default":
        parts.append("default-syntax-highlighting")
    elif variant == "default-colorful":
        parts.append("default-syntax-highlighting-colorful-status-bar")
    return "-".join(parts) + ".json"


def system_accent() -> str | None:
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


def resolve_accents(spec: str | None, use_system: bool) -> list[str]:
    if spec == "all":
        return list(ACCENT_NAMES)
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
        accent = system_accent()
        if accent and accent not in accents:
            accents.append(accent)
    return accents


def relative_luminance(color: str) -> float:
    r, g, b, _ = parse_color(color)

    def channel(value: float) -> float:
        value /= 255
        return value / 12.92 if value <= 0.03928 else cast(float, ((value + 0.055) / 1.055) ** 2.4)

    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)


def contrast(color_a: str, color_b: str) -> float:
    lum_a, lum_b = relative_luminance(color_a), relative_luminance(color_b)
    lighter, darker = max(lum_a, lum_b), min(lum_a, lum_b)
    return (lighter + 0.05) / (darker + 0.05)


def theme_label(mode: str, accent: str) -> str:
    kind = MODE_LABELS[mode]
    if accent == "blue":
        return f"Adwaita {kind}"
    return f"Adwaita {ACCENT_LABELS[accent]} {kind}"


def build_theme(mode: str, accent: str, variant: str, high_contrast: bool = False) -> Theme:
    """variant 取值：builder、colorful、default、default-colorful。"""
    scheme = tokens.editor_colors(mode)
    palette = Palette(mode, accent=accent, high_contrast=high_contrast, scheme=scheme)
    colorful = variant in ("colorful", "default-colorful")
    default_syntax = variant in ("default", "default-colorful")

    if high_contrast:
        label = f"Adwaita {MODE_LABELS[mode]} 高对比度"
    else:
        label = theme_label(mode, accent)
        if default_syntax:
            label += " · 默认语法高亮"
        if colorful:
            label += " · 彩色状态栏"

    token_colors: list[tokens.TokenRule]
    if default_syntax:
        token_colors = cast(list[tokens.TokenRule], json.loads((DEFAULTS / f"{mode}.json").read_text())["tokenColors"])
    else:
        token_colors = tokens.token_colors(mode)

    return {
        "$schema": "vscode://schemas/color-theme",
        "name": label,
        "type": mode,
        "semanticHighlighting": True,
        "colors": mapping.build_ui_colors(palette, colorful_status_bar=colorful),
        "tokenColors": token_colors,
        "semanticTokenColors": tokens.semantic_token_colors(mode),
    }


def build_plan(accents: list[str]) -> list[ThemeRequest]:
    plan: list[ThemeRequest] = []
    for mode in ("dark", "light"):
        for accent in accents:
            variants = ["builder", "colorful", "default", "default-colorful"] if accent == "blue" else ["builder"]
            for variant in variants:
                plan.append({"mode": mode, "accent": accent, "variant": variant, "hc": False})
        plan.append({"mode": mode, "accent": "blue", "variant": "builder", "hc": True})
    return plan


def write_themes(plan: list[ThemeRequest], watch: bool = False) -> list[ThemeEntry]:
    THEMES.mkdir(exist_ok=True)
    # 先生成所有内容，生成器失败时保留原有主题文件。
    prepared = [(item, build_theme(item["mode"], item["accent"], item["variant"], item["hc"])) for item in plan]
    written: set[Path] = set()
    entries: list[ThemeEntry] = []
    for item, theme in prepared:
        path = THEMES / theme_filename(item["mode"], item["accent"], item["variant"], item["hc"])
        path.write_text(json.dumps(theme, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        written.add(path)
        if item["hc"]:
            ui_theme = "hc-black" if item["mode"] == "dark" else "hc-light"
        else:
            ui_theme = "vs-dark" if item["mode"] == "dark" else "vs"
        entry: WatchThemeEntry = {
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


def update_package_json(entries: list[ThemeEntry]) -> None:
    path = ROOT / "package.json"
    manifest = json.loads(path.read_text())
    manifest["contributes"]["themes"] = entries
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")


def load_known_keys() -> tuple[set[str] | None, set[str] | None]:
    """返回（注册表键集合, 内置主题键集合）。"""
    registry: set[str] | None = None
    builtin: set[str] | None = None
    registry_path = DEFAULTS / "registry_keys.json"
    builtin_path = DEFAULTS / "builtin_keys.json"
    if registry_path.exists():
        registry = set(json.loads(registry_path.read_text()))
    if builtin_path.exists():
        builtin = set(json.loads(builtin_path.read_text()))
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


def check() -> int:
    failures: int = 0
    registry, builtin = load_known_keys()
    our_keys: set[str] = set()
    labels: list[str] = []
    for path in sorted(THEMES.glob("*.json")):
        theme = json.loads(path.read_text())
        labels.append(theme["name"])
        our_keys |= set(theme["colors"])

        for key, value in theme["colors"].items():
            try:
                parse_color(value)
            except ValueError as error:
                print(f"失败 {path.name}: {key} 的颜色无效: {error}")
                failures += 1
        for rule in theme["tokenColors"]:
            if not rule.get("scope") or not rule.get("settings"):
                print(f"失败 {path.name}: 语法规则不完整 {rule}")
                failures += 1
        for name, value in theme.get("semanticTokenColors", {}).items():
            if value is None:
                print(f"失败 {path.name}: 语义标记 {name} 未定义颜色")
                failures += 1
    if len(labels) != len(set(labels)):
        print("失败 主题名称重复")
        failures += 1

    # 产品图标主题
    icons_path = ROOT / "product-icons" / "adwaita.json"
    if icons_path.exists():
        icons = json.loads(icons_path.read_text())
        fonts = icons.get("fonts", [])
        definitions = icons.get("iconDefinitions", {})
        if not fonts or not definitions:
            print("失败 product-icons/adwaita.json: 缺少 fonts 或 iconDefinitions")
            failures += 1
        for font in fonts:
            for source in font.get("src", []):
                target = (icons_path.parent / source["path"]).resolve()
                if not target.exists():
                    print(f"失败 product-icons：缺少 {source['path']}")
                    failures += 1
        if len({d.get("fontCharacter") for d in definitions.values()}) != len(definitions):
            print("失败 product-icons: 图标字形重复")
            failures += 1
        print(f"产品图标：{len(definitions)} 个字形，{len(fonts)} 个字体")

    # 对每个生成的主题做对比度检查（含强调色、变体、高对比度）。
    checks: list[tuple[str, str, float]] = [
        ("editor.foreground", "editor.background", 4.5),
        ("button.foreground", "button.background", 2.5),  # GNOME 黄色的对比度约为 2.8
        ("textLink.foreground", "editor.background", 3.0),
        ("gitDecoration.deletedResourceForeground", "editor.background", 3.0),
        ("descriptionForeground", "editor.background", 2.5),
    ]
    failed_themes: int = 0
    for path in sorted(THEMES.glob("*.json")):
        theme = json.loads(path.read_text())
        colors = theme["colors"]
        problems: list[str] = []
        for fg_key, bg_key, minimum in checks:
            ratio = contrast(colors[fg_key], colors[bg_key])
            if ratio < minimum:
                problems.append(f"{fg_key}/{bg_key} {ratio:.2f} < {minimum}")
        status = "通过" if not problems else "失败"
        print(f"{status} {theme['name']}")
        for problem in problems:
            print(f"       {problem}")
            failures += 1
            failed_themes += 1
    print(f"对比度：{len(list(THEMES.glob('*.json'))) - failed_themes} 个主题通过，{failed_themes} 个主题存在问题")

    if registry is None or builtin is None:
        print("提示：运行 update_defaults.py 刷新 VS Code 颜色键表")
    else:
        allowed = registry | builtin | LEGACY_KEYS
        unknown = sorted(our_keys - allowed)
        missing = sorted(builtin - our_keys)
        print(f"\n颜色键：已定义 {len(our_keys)} 个 | 未覆盖内置键 {len(missing)} 个 | 未知键 {len(unknown)} 个")
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--accents", help="使用 'all'、'system' 或以逗号分隔的强调色列表")
    parser.add_argument("--no-system", action="store_true", help="不读取系统强调色")
    parser.add_argument("--check", action="store_true", help="仅校验已生成的主题")
    parser.add_argument(
        "--watch",
        action="store_true",
        help="添加未公开的 _watch 标记，让 VS Code 在保存主题 JSON 时重新加载（仅用于开发）",
    )
    args = parser.parse_args()

    if args.check:
        return check()

    accents = resolve_accents(args.accents if args.accents != "system" else None, not args.no_system)
    print(f"强调色： {', '.join(accents)}")
    entries = write_themes(build_plan(accents), watch=args.watch)
    if args.watch:
        print("监视模式：已为主题条目添加 _watch 标记")
    update_package_json(entries)
    for entry in entries:
        print(f"  {entry['uiTheme']:9} {entry['label']}")
    print(f"已将 {len(entries)} 个主题写入 {THEMES.relative_to(ROOT)}/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
