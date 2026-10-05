#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""AdwCode 生成器测试：颜色运算、调色板、语法与主题。

运行：python3.14t -m unittest discover -s tests -p 'test_*.py'
"""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
import unittest
import tempfile
import zipfile
from unittest.mock import patch
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).parent.parent
SRC = ROOT / "src"
sys.path.insert(0, str(SRC))

import tokens
from palette import ACCENT_NAMES, Palette, mix, over, parse_color, rgba, to_hex

THEMES = ROOT / "themes"
REGISTRY = cast(list[str], json.loads((SRC / "vscode_defaults" / "registry_keys.json").read_text(encoding="utf-8")))


class ColorMathTest(unittest.TestCase):
    def test_parse_hex_forms(self) -> None:
        self.assertEqual(parse_color("#fff"), (255, 255, 255, 1.0))
        self.assertEqual(parse_color("#000006"), (0, 0, 6, 1.0))
        self.assertEqual(parse_color("#00000680"), (0, 0, 6, 128 / 255))

    def test_parse_rgb_forms(self) -> None:
        self.assertEqual(parse_color("rgb(0 0 6 / 80%)"), (0, 0, 6, 0.8))
        self.assertEqual(parse_color("rgb(255 255 255 / 8%)"), (255, 255, 255, 0.08))
        self.assertEqual(parse_color("rgb(10 20 30)"), (10, 20, 30, 1.0))

    def test_invalid_colors_rejected(self) -> None:
        for color in ("#12345", "#123456789", "#ggg", "rgb(256 0 0)", "rgb(0 0 0 / 101%)", "rgb(0 0 0 / 1.1)"):
            with self.subTest(color=color), self.assertRaises(ValueError):
                parse_color(color)

    def test_round_trip(self) -> None:
        self.assertEqual(to_hex(0, 0, 6, 1.0), "#000006")
        self.assertEqual(to_hex(0, 0, 6, 0.8), "#000006cc")

    def test_mix_and_over(self) -> None:
        self.assertEqual(mix("#ffffff", "#000000", 0.5), "#808080")
        self.assertEqual(over("#ffffff80", "#000000"), "#808080")
        self.assertEqual(over("#ffffff", "#000000"), "#ffffff")
        self.assertEqual(rgba("#3584e4", 0.5), "#3584e480")

    def test_mix_rejects_translucent(self) -> None:
        with self.assertRaises(ValueError):
            mix("#ffffff80", "#000000", 0.5)


class PaletteTest(unittest.TestCase):
    def test_all_roles_parse(self) -> None:
        for mode in ("dark", "light"):
            for accent in ACCENT_NAMES:
                for high_contrast in (False, True):
                    palette = Palette(mode, accent=accent, high_contrast=high_contrast)
                    for role, value in palette.roles().items():
                        parse_color(value)

    def test_light_foreground_is_composited(self) -> None:
        palette = Palette("light")
        # 半透明的 rgb(0 0 6 / 80%) 必须被合成为不透明色
        self.assertEqual(len(palette["fg_view"]), 7)
        self.assertNotEqual(palette["fg_view"], "#000006")

    def test_high_contrast_borders(self) -> None:
        normal = Palette("dark")
        high = Palette("dark", high_contrast=True)
        self.assertGreater(
            parse_color(high["border"])[3], parse_color(normal["border"])[3]
        )
        self.assertEqual(parse_color(high["fg_view"])[3], 1.0)

    def test_accents_differ(self) -> None:
        self.assertNotEqual(Palette("dark", "blue")["accent_bg"], Palette("dark", "teal")["accent_bg"])
        self.assertNotEqual(
            Palette("light", "blue")["accent_standalone"],
            Palette("dark", "blue")["accent_standalone"],
        )

    def test_unknown_inputs_rejected(self) -> None:
        with self.assertRaises(ValueError):
            Palette("dark", accent="chartreuse")
        with self.assertRaises(ValueError):
            Palette("sepia")


class TokensTest(unittest.TestCase):
    def test_editor_colors(self) -> None:
        for mode, expected_bg in (("dark", "#1d1d20"), ("light", "#ffffff")):
            colors = tokens.editor_colors(mode)
            self.assertEqual(colors["text_bg"], expected_bg)
            for value in colors.values():
                if value is not None:
                    parse_color(value)

    def test_token_rules_are_complete(self) -> None:
        for mode in ("dark", "light"):
            rules = tokens.token_colors(mode)
            self.assertGreater(len(rules), 30)
            for rule in rules:
                self.assertTrue(rule["scope"])
                self.assertIn("settings", rule)
                self.assertIn("fontStyle", rule["settings"])

    def test_semantic_colors_have_no_nulls(self) -> None:
        for mode in ("dark", "light"):
            for name, value in tokens.semantic_token_colors(mode).items():
                self.assertIsNotNone(value, name)
                assert value is not None
                parse_color(value)

    def test_current_scheme_uses_def_statement(self) -> None:
        # 当前 GtkSourceView 已将 def:keyword 更名；别名必须继续被容忍
        for mode in ("dark", "light"):
            self.assertIn("def:statement", tokens.load_scheme(mode)["styles"])
            self.assertNotIn("def:keyword", tokens.load_scheme(mode)["styles"])


class ThemeTest(unittest.TestCase):
    def themes(self) -> list[dict[str, Any]]:
        return [json.loads(path.read_text(encoding="utf-8")) for path in sorted(THEMES.glob("*.json"))]

    def test_themes_exist(self) -> None:
        self.assertGreaterEqual(len(self.themes()), 10)

    def test_labels_unique(self) -> None:
        labels = [theme["name"] for theme in self.themes()]
        self.assertEqual(len(labels), len(set(labels)))

    def test_colors_are_registered(self) -> None:
        from build import LEGACY_KEYS

        builtin = set(json.loads((SRC / "vscode_defaults" / "builtin_keys.json").read_text(encoding="utf-8")))
        known = set(REGISTRY) | builtin | LEGACY_KEYS
        for theme in self.themes():
            for key in theme["colors"]:
                self.assertIn(key, known, f"{theme['name']}: {key}")

    def test_semantic_and_tokens(self) -> None:
        for theme in self.themes():
            self.assertTrue(theme["semanticHighlighting"])
            for value in theme["semanticTokenColors"].values():
                parse_color(value)
            for rule in theme["tokenColors"]:
                self.assertTrue(rule["scope"])
                self.assertTrue(rule["settings"])

    def test_contrast_keys_only_in_high_contrast(self) -> None:
        for theme in self.themes():
            hc = "高对比度" in theme["name"]
            for key in ("contrastBorder", "contrastActiveBorder"):
                self.assertEqual(
                    key in theme["colors"],
                    hc,
                    f"{theme['name']}: {key} 只能在高对比度主题中定义",
                )

    def test_labels_are_chinese(self) -> None:
        """主题标签必须能被扩展的正则解析（Adwaita [强调色] 深色/浅色[后缀]）。"""
        for theme in self.themes():
            name = theme["name"]
            self.assertTrue(name.startswith("Adwaita "), name)
            self.assertIn("深色" if theme["type"] == "dark" else "浅色", name)
            # 默认蓝色不带强调色前缀，其余强调色必须出现在标签里
            self.assertNotIn("蓝色", name)
            if theme["colors"]["button.background"] != Palette("dark", "blue")["accent_bg"]:
                self.assertRegex(name, r"^Adwaita (青色|绿色|黄色|橙色|红色|粉色|紫色|石板灰) ")

    def test_ui_theme_matches_type(self) -> None:
        manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        entries = {entry["label"]: entry for entry in manifest["contributes"]["themes"]}
        for theme in self.themes():
            entry = entries[theme["name"]]
            if theme["type"] == "dark":
                self.assertIn(entry["uiTheme"], ("vs-dark", "hc-black"))
            else:
                self.assertIn(entry["uiTheme"], ("vs", "hc-light"))
            self.assertTrue((ROOT / entry["path"].removeprefix("./")).exists())

    def test_slugs_are_safe(self) -> None:
        for path in THEMES.glob("*.json"):
            self.assertRegex(path.name, r"^[a-z0-9-]+\.json$")


class GeneratorSafetyTest(unittest.TestCase):
    def test_jsonc_preserves_string_contents(self) -> None:
        from update_defaults import strip_jsonc
        data = json.loads(strip_jsonc('{"text": ",} // /* ", /* 注释 */ "list": [1, 2,],}'))
        self.assertEqual(data, {"text": ",} // /* ", "list": [1, 2]})

    def test_include_cycle_rejected(self) -> None:
        import update_defaults
        with patch.dict(update_defaults._cache, {"a": {"include": "b.json"}, "b": {"include": "a.json"}}):
            with self.assertRaisesRegex(ValueError, "include 循环"):
                update_defaults.resolve_token_colors("a")

    def test_accents_deduplicated(self) -> None:
        from build import resolve_accents
        self.assertEqual(resolve_accents("blue,teal,blue", False), ["blue", "teal"])
        with self.assertRaises(SystemExit):
            resolve_accents(" , ", False)

    def test_generation_failure_preserves_existing_themes(self) -> None:
        import build
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            existing = folder / "old.json"
            existing.write_text("原有主题", encoding="utf-8")
            with patch.object(build, "THEMES", folder), patch.object(build, "build_theme", side_effect=ValueError("生成失败")):
                with self.assertRaises(ValueError):
                    build.write_themes(build.build_plan(["blue"]))
            self.assertEqual(existing.read_text(encoding="utf-8"), "原有主题")


class ExtensionStatusTest(unittest.TestCase):
    def test_missing_development_tools_fail(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SRC / "dev.py"), "typecheck"],
            env={"PATH": ""}, capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("缺少开发工具：ty、tsc、node", result.stdout)

    def test_vsix_content_types(self) -> None:
        import package

        root = ET.fromstring(package.CONTENT_TYPES)
        entries = {entry.attrib["Extension"]: entry.attrib["ContentType"] for entry in root}
        self.assertTrue(all(not extension.startswith(".") for extension in entries))
        self.assertEqual(entries["css"], "text/css")
        self.assertEqual(entries["json"], "application/json")

    def test_vsix_xml_escapes_metadata(self) -> None:
        import package
        manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        manifest["description"] = '说明 <示例> & "引号"'
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
            icon = folder / manifest["icon"]
            icon.parent.mkdir(parents=True)
            icon.write_bytes((ROOT / manifest["icon"]).read_bytes())
            with patch.object(package, "ROOT", folder):
                package.main()
            with zipfile.ZipFile(folder / f"{manifest['name']}-{manifest['version']}.vsix") as archive:
                xml = ET.fromstring(archive.read("extension.vsixmanifest"))
                description = xml.find("{*}Metadata/{*}Description")
                assert description is not None
                self.assertEqual(description.text, manifest["description"])
                icon_metadata = xml.find("{*}Metadata/{*}Icon")
                assert icon_metadata is not None
                icon_uri = "extension/" + manifest["icon"]
                self.assertEqual(icon_metadata.text, icon_uri)
                assets = xml.findall("{*}Assets/{*}Asset")
                registered_icons = [asset for asset in assets if asset.attrib["Type"] == "Microsoft.VisualStudio.Services.Icons.Default"]
                self.assertEqual(len(registered_icons), 1)
                self.assertEqual(registered_icons[0].attrib["Path"], icon_uri)
                self.assertEqual(archive.read(icon_uri), icon.read_bytes())
            # 配置了图标却未纳入包，或引用仓库之外的文件时必须失败。
            for bad_icon in ("missing.png", "../outside.png", str(icon)):
                manifest["icon"] = bad_icon
                (folder / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
                with patch.object(package, "ROOT", folder), self.assertRaises(ValueError):
                    package.main()

    def test_recommended_settings_recovery(self) -> None:
        node = shutil.which("node")
        if node is None:
            self.skipTest("未安装 Node.js")
        result = subprocess.run([node, str(ROOT / "tests/test_extension_settings.cjs")], capture_output=True, text=True, timeout=10, check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_offline_status_panel(self) -> None:
        node = shutil.which("node")
        if node is None:
            self.skipTest("未安装 Node.js，跳过扩展状态面板测试")
        result = subprocess.run(
            [node, str(ROOT / "tests" / "test_extension_status.cjs")],
            capture_output=True, text=True, timeout=10, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class CssTest(unittest.TestCase):
    def test_css_files_parse(self) -> None:
        import check_css

        sheets = sorted((ROOT / "extras").glob("*.css"))
        self.assertTrue(sheets)
        for sheet in sheets:
            selectors, declarations = check_css.selectors_and_declarations(sheet.read_text(encoding="utf-8"))
            self.assertTrue(selectors.strip(), sheet.name)
            self.assertTrue(declarations.strip(), sheet.name)

    def test_referenced_variables_are_defined(self) -> None:
        import check_css

        vscode_css, _ = check_css.find_vscode_assets(None)
        vscode_defined: set[str] = check_css.theme_variables()
        if vscode_css is not None:
            vscode_defined |= set(
                check_css.VAR_DEF_RE.findall(
                    vscode_css.read_text(encoding="utf-8", errors="ignore")
                )
            )
        for sheet in sorted((ROOT / "extras").glob("*.css")):
            text = sheet.read_text(encoding="utf-8")
            _, declarations = check_css.selectors_and_declarations(text)
            referenced = set(check_css.VAR_REF_RE.findall(declarations))
            defined = set(check_css.VAR_DEF_RE.findall(text))
            missing = sorted(referenced - defined - vscode_defined)
            self.assertFalse(missing, f"{sheet.name}: 未定义的变量 {missing}")

    def test_vscode_selectors_still_exist(self) -> None:
        import check_css

        vscode_css, _ = check_css.find_vscode_assets(None)
        if vscode_css is None:
            self.skipTest("未找到已安装的 VS Code")
        self.assertEqual(check_css.check(None), 0)


class ProductIconSourcesTests(unittest.TestCase):
    def test_imported_sources_match_assets_and_mappings(self) -> None:
        """上游资产保持原字节；码点、字体与所有别名必须能对应到来源记录。"""
        folder = ROOT / "product-icons"
        sources = json.loads((folder / "sources.json").read_text(encoding="utf-8"))
        theme = json.loads((folder / "adwaita.json").read_text(encoding="utf-8"))
        fonts = {font["id"]: font for font in theme["fonts"]}
        self.assertEqual(len(fonts), len(theme["fonts"]))
        imported_ids = {font["id"] for font in sources["fonts"] if font.get("enabled", True)}
        mapped: set[str] = set()
        for font in sources["fonts"]:
            self.assertRegex(font["revision"], r"^[0-9a-f]{40}$")
            self.assertTrue(font["license"])
            self.assertTrue(font["attribution"])
            enabled = font.get("enabled", True)
            if enabled:
                self.assertEqual(fonts[font["id"]]["src"][0]["path"], "./" + font["output"])
            else:
                self.assertNotIn(font["id"], fonts)
            self.assertTrue((folder / font["output"]).is_file())
            codepoints: set[int] = set()
            for entry in font["glyphs"]:
                path = folder / entry["file"]
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), entry["sha256"], entry["file"])
                self.assertEqual(Path(entry["upstream_path"]).name, path.name)
                self.assertEqual(ET.parse(path).getroot().tag, "{http://www.w3.org/2000/svg}svg")
                preview = folder / "rendered" / font["id"] / (entry["codepoint"] + ".svg")
                self.assertIn(font["license"], preview.read_text(encoding="utf-8"))
                preview_root = ET.parse(preview).getroot()
                self.assertEqual(preview_root.attrib["viewBox"], "0 0 16 16")
                preview_paths = preview_root.findall("{http://www.w3.org/2000/svg}path")
                self.assertTrue(preview_paths)
                self.assertTrue(all(path.attrib.get("d") for path in preview_paths))
                codepoint = int(entry["codepoint"], 16)
                self.assertTrue(0xE000 <= codepoint <= 0xF8FF)
                self.assertNotIn(codepoint, codepoints)
                codepoints.add(codepoint)
                self.assertTrue(entry["icons"])
                if not enabled:
                    continue
                for icon in entry["icons"]:
                    self.assertNotIn(icon, mapped)
                    mapped.add(icon)
                    self.assertEqual(theme["iconDefinitions"][icon], {
                        "fontId": font["id"], "fontCharacter": "\\" + entry["codepoint"],
                    })
        self.assertEqual(mapped, {
            icon for icon, definition in theme["iconDefinitions"].items()
            if definition["fontId"] in imported_ids
        })

    def test_adwaita_equivalents_take_priority(self) -> None:
        """官方已有对应字形时采用 Adwaita；备用 MoreWaita 不参与运行时加载。"""
        folder = ROOT / "product-icons"
        sources = json.loads((folder / "sources.json").read_text(encoding="utf-8"))
        theme = json.loads((folder / "adwaita.json").read_text(encoding="utf-8"))
        priority = ["adwcode-adwaita", "adwcode-builder", "adwcode-morewaita"]
        self.assertEqual(sources["source_priority"], priority)
        self.assertEqual([font["id"] for font in sources["fonts"]], priority)
        for icon in ("terminal", "extensions", "debug-continue", "debug-stop", "package", "tools",
                     "root-folder", "chrome-close", "chrome-maximize", "chrome-minimize", "chrome-restore"):
            self.assertEqual(theme["iconDefinitions"][icon]["fontId"], "adwcode-adwaita", icon)
        self.assertNotIn("adwcode-morewaita", {font["id"] for font in theme["fonts"]})


if __name__ == "__main__":
    unittest.main()
