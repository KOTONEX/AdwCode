#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode 贡献者
"""AdwCode 生成器测试：颜色运算、调色板、语法与主题。

运行：python3.14t -m unittest discover -s 测试 -p 'test_*.py'
"""

from __future__ import annotations

import hashlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile
from contextlib import redirect_stdout
from pathlib import Path
from typing import Any, cast
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent.parent / "源码"))

import 语法映射
from 调色板 import ACCENT_NAMES, 叠加颜色, 合成透明颜色, 混色, 解析颜色, 调色板对象, 转为十六进制

ROOT = Path(__file__).parent.parent
SRC = ROOT / "源码"
THEMES = ROOT / "主题"
REGISTRY = cast(
    list[str],
    json.loads((SRC / "VSCode默认数据" / "registry_keys.json").read_text(encoding="utf-8")),
)


class ColorMathTest(unittest.TestCase):
    def test_parse_hex_forms(self) -> None:
        self.assertEqual(解析颜色("#fff"), (255, 255, 255, 1.0))
        self.assertEqual(解析颜色("#000006"), (0, 0, 6, 1.0))
        self.assertEqual(解析颜色("#00000680"), (0, 0, 6, 128 / 255))

    def test_parse_rgb_forms(self) -> None:
        self.assertEqual(解析颜色("rgb(0 0 6 / 80%)"), (0, 0, 6, 0.8))
        self.assertEqual(解析颜色("rgb(255 255 255 / 8%)"), (255, 255, 255, 0.08))
        self.assertEqual(解析颜色("rgb(10 20 30)"), (10, 20, 30, 1.0))

    def test_invalid_colors_rejected(self) -> None:
        for color in (
            "#12345",
            "#123456789",
            "#ggg",
            "rgb(256 0 0)",
            "rgb(0 0 0 / 101%)",
            "rgb(0 0 0 / 1.1)",
        ):
            with self.subTest(color=color), self.assertRaises(ValueError):
                解析颜色(color)

    def test_round_trip(self) -> None:
        self.assertEqual(转为十六进制(0, 0, 6, 1.0), "#000006")
        self.assertEqual(转为十六进制(0, 0, 6, 0.8), "#000006cc")

    def test_mix_and_over(self) -> None:
        self.assertEqual(混色("#ffffff", "#000000", 0.5), "#808080")
        self.assertEqual(混色("#ffffff", "#000000", 0.25), "#404040")
        self.assertEqual(叠加颜色("#ffffff80", "#000000"), "#808080")
        self.assertEqual(叠加颜色("#ffffff", "#000000"), "#ffffff")
        self.assertEqual(合成透明颜色("#3584e4", 0.5), "#3584e480")

    def test_mix_rejects_translucent(self) -> None:
        with self.assertRaises(ValueError):
            混色("#ffffff80", "#000000", 0.5)


class PaletteTest(unittest.TestCase):
    def test_accent_interaction_contrast(self) -> None:
        from 生成主题 import 对比度

        for mode in ("dark", "light"):
            for accent in ACCENT_NAMES:
                调色板 = 调色板对象(mode, accent=accent)
                for role in ("accent_hover", "accent_active"):
                    with self.subTest(mode=mode, accent=accent, role=role):
                        self.assertGreaterEqual(对比度(调色板["accent_fg"], 调色板[role]), 2.0)

    def test_all_roles_parse(self) -> None:
        for mode in ("dark", "light"):
            for accent in ACCENT_NAMES:
                for high_contrast in (False, True):
                    调色板 = 调色板对象(mode, accent=accent, high_contrast=high_contrast)
                    for role, value in 调色板.roles().items():
                        解析颜色(value)

    def test_light_foreground_is_composited(self) -> None:
        调色板 = 调色板对象("light")
        # 半透明的 rgb(0 0 6 / 80%) 必须被合成为不透明色
        self.assertEqual(len(调色板["fg_view"]), 7)
        self.assertNotEqual(调色板["fg_view"], "#000006")

    def test_high_contrast_borders(self) -> None:
        normal = 调色板对象("dark")
        high = 调色板对象("dark", high_contrast=True)
        self.assertGreater(解析颜色(high["border"])[3], 解析颜色(normal["border"])[3])
        self.assertEqual(解析颜色(high["fg_view"])[3], 1.0)

    def test_unknown_inputs_rejected(self) -> None:
        with self.assertRaises(ValueError):
            调色板对象("dark", accent="chartreuse")
        with self.assertRaises(ValueError):
            调色板对象("sepia")


class TokensTest(unittest.TestCase):
    def test_editor_colors(self) -> None:
        for mode, expected_bg in (("dark", "#1d1d20"), ("light", "#ffffff")):
            colors = 语法映射.编辑器颜色(mode)
            self.assertEqual(colors["text_bg"], expected_bg)
            for value in colors.values():
                if value is not None:
                    解析颜色(value)

    def test_token_rules_are_complete(self) -> None:
        for mode in ("dark", "light"):
            rules = 语法映射.语法颜色(mode)
            self.assertGreater(len(rules), 30)
            for rule in rules:
                self.assertTrue(rule["scope"])
                self.assertIn("settings", rule)
                self.assertIn("fontStyle", rule["settings"])

    def test_semantic_colors_have_no_nulls(self) -> None:
        for mode in ("dark", "light"):
            for name, value in 语法映射.语义标记颜色(mode).items():
                self.assertIsNotNone(value, name)
                assert value is not None
                解析颜色(value)

    def test_current_scheme_uses_def_statement(self) -> None:
        # 当前 GtkSourceView 已将 def:keyword 更名，不保留旧方案别名
        for mode in ("dark", "light"):
            self.assertIn("def:statement", 语法映射.加载样式方案(mode)["styles"])
            self.assertNotIn("def:keyword", 语法映射.加载样式方案(mode)["styles"])


class ThemeTest(unittest.TestCase):
    def test_不再生成默认语法高亮变体(self) -> None:
        from 生成主题 import 构建计划, 生成主题对象

        plan = 构建计划(["blue"])
        self.assertEqual({item["variant"] for item in plan}, {"builder", "colorful"})
        for variant in ("default", "default-colorful"):
            with self.subTest(variant=variant), self.assertRaises(ValueError):
                生成主题对象("light", "blue", variant)

    def test_浅色标签条带区分选中与编辑区(self) -> None:
        from 生成主题 import 对比度, 生成主题对象

        colors = 生成主题对象("light", "blue", "builder")["colors"]
        self.assertNotEqual(colors["editorGroupHeader.tabsBackground"], colors["editor.background"])
        self.assertNotEqual(colors["tab.activeBackground"], colors["tab.inactiveBackground"])
        self.assertEqual(colors["tab.activeBackground"], colors["editor.background"])
        self.assertGreaterEqual(
            对比度(colors["tab.inactiveForeground"], colors["tab.inactiveBackground"]), 4.5
        )
        self.assertNotEqual(colors["editorGroupHeader.tabsBorder"], "#00000000")

    def test_固定蓝色并拒绝其他强调色(self) -> None:
        from 生成主题 import 构建计划

        self.assertEqual(len(构建计划(["blue"])), 6)
        for accent in ("teal", "green", "yellow", "orange", "red", "pink", "purple", "slate"):
            with self.subTest(accent=accent), self.assertRaises(ValueError):
                调色板对象("dark", accent=accent)

    def test_colorful_status_bar_hover_foregrounds(self) -> None:
        from 生成主题 import 对比度, 生成主题对象

        for mode in ("dark", "light"):
            colors = 生成主题对象(mode, "blue", "colorful")["colors"]
            self.assertEqual(
                colors["statusBarItem.hoverForeground"], colors["statusBar.foreground"]
            )
            self.assertGreaterEqual(
                对比度(
                    colors["statusBarItem.hoverForeground"], colors["statusBarItem.hoverBackground"]
                ),
                2.5,
            )
            self.assertGreaterEqual(
                对比度(
                    colors["statusBarItem.warningHoverForeground"],
                    colors["statusBarItem.warningHoverBackground"],
                ),
                4.5,
            )

    def themes(self) -> list[dict[str, Any]]:
        return [
            json.loads(path.read_text(encoding="utf-8")) for path in sorted(THEMES.glob("*.json"))
        ]

    def test_themes_exist(self) -> None:
        self.assertGreaterEqual(len(self.themes()), 6)

    def test_labels_unique(self) -> None:
        labels = [theme["name"] for theme in self.themes()]
        self.assertEqual(len(labels), len(set(labels)))

    def test_colors_are_registered(self) -> None:
        from 生成主题 import 补充颜色键

        builtin = set(
            json.loads((SRC / "VSCode默认数据" / "builtin_keys.json").read_text(encoding="utf-8"))
        )
        known = set(REGISTRY) | builtin | 补充颜色键
        for theme in self.themes():
            for key in theme["colors"]:
                self.assertIn(key, known, f"{theme['name']}: {key}")

    def test_semantic_and_tokens(self) -> None:
        for theme in self.themes():
            self.assertTrue(theme["semanticHighlighting"])
            for value in theme["semanticTokenColors"].values():
                解析颜色(value)
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
        """主题标签必须能被扩展的正则解析（AdwCode 深色/浅色[后缀]）。"""
        for theme in self.themes():
            name = theme["name"]
            self.assertTrue(name.startswith("AdwCode "), name)
            self.assertIn("深色" if theme["type"] == "dark" else "浅色", name)
            # 固定蓝色不带强调色前缀
            self.assertNotIn("蓝色", name)

        # 覆盖按需构建的蓝色主题变体，防止仅默认主题完成更名。
        from 生成主题 import 主题文件名, 构建计划, 生成主题对象

        for request in 构建计划(list(ACCENT_NAMES)):
            theme = 生成主题对象(
                request["mode"], request["accent"], request["variant"], request["hc"]
            )
            filename = 主题文件名(
                request["mode"], request["accent"], request["variant"], request["hc"]
            )
            self.assertTrue(theme["name"].startswith("AdwCode "), theme["name"])
            self.assertTrue(filename.startswith("adwcode-"), filename)

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
            self.assertRegex(path.name, r"^adwcode-[a-z0-9\u4e00-\u9fff-]+\.json$")


class GeneratorSafetyTest(unittest.TestCase):
    def test_missing_key_tables_fail(self) -> None:
        import 生成主题

        for known in ((None, None), (None, set()), (set(), None)):
            with (
                self.subTest(known=known),
                patch.object(生成主题, "加载颜色键表", return_value=known),
                redirect_stdout(io.StringIO()),
            ):
                self.assertEqual(生成主题.校验(), 1)

    def test_check_rejects_missing_assets_and_non_hex_colors(self) -> None:
        import 生成主题

        for case in (
            "empty",
            "theme",
            "icons",
            "icon_registration",
            "font_source",
            "font_directory",
            "label",
            "type",
            "ui_color",
            "token_color",
            "semantic_color",
        ):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                shutil.copytree(THEMES, folder / "主题")
                shutil.copytree(ROOT / "产品图标", folder / "产品图标")
                shutil.copy2(ROOT / "package.json", folder / "package.json")
                paths = sorted((folder / "主题").glob("*.json"))
                if case == "empty":
                    for path in paths:
                        path.unlink()
                elif case == "theme":
                    paths[0].unlink()
                elif case == "icons":
                    (folder / "产品图标/adwcode.json").unlink()
                elif case == "icon_registration":
                    manifest_path = folder / "package.json"
                    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                    manifest["contributes"]["productIconThemes"][0]["path"] = (
                        "./产品图标/missing.json"
                    )
                    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
                elif case in ("font_source", "font_directory"):
                    icon_path = folder / "产品图标/adwcode.json"
                    icons = json.loads(icon_path.read_text(encoding="utf-8"))
                    icons["fonts"][0]["src"] = [] if case == "font_source" else [{"path": "."}]
                    icon_path.write_text(json.dumps(icons), encoding="utf-8")
                else:
                    if case == "type":
                        paths[0] = next(path for path in paths if "浅色" in path.name)
                    theme = json.loads(paths[0].read_text(encoding="utf-8"))
                    if case == "label":
                        theme["name"] = "未注册的主题标签"
                    elif case == "type":
                        theme["type"] = "未知类型"
                    elif case == "ui_color":
                        theme["colors"]["badge.background"] = "rgb(0 0 0 / 50%)"
                    elif case == "token_color":
                        theme["tokenColors"][0]["settings"]["foreground"] = "rgb(0 0 0)"
                    else:
                        theme["semanticTokenColors"]["class"] = "rgb(0 0 0)"
                    paths[0].write_text(json.dumps(theme), encoding="utf-8")
                with (
                    patch.object(生成主题, "ROOT", folder),
                    patch.object(生成主题, "THEMES", folder / "主题"),
                    redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(生成主题.校验(), 1)

    def test_jsonc_preserves_string_contents(self) -> None:
        from 更新默认数据 import 清理JSON注释

        data = json.loads(清理JSON注释('{"text": ",} // /* ", /* 注释 */ "list": [1, 2,],}'))
        self.assertEqual(data, {"text": ",} // /* ", "list": [1, 2]})

    def test_defaults_download_failure_preserves_existing_data(self) -> None:
        import 更新默认数据

        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            files = ("builtin_keys.json", "registry_keys.json")
            for name in files:
                (folder / name).write_text("原有数据", encoding="utf-8")
            with (
                patch.object(更新默认数据, "OUT", folder),
                patch.object(更新默认数据, "获取默认主题", return_value={"tokenColors": []}),
                patch.object(
                    更新默认数据.urllib.request, "urlopen", side_effect=OSError("下载失败")
                ),
                redirect_stdout(io.StringIO()),
                self.assertRaises(OSError),
            ):
                更新默认数据.入口()
            for name in files:
                self.assertEqual((folder / name).read_text(encoding="utf-8"), "原有数据")

    def test_generation_failure_preserves_existing_themes(self) -> None:
        import 生成主题

        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            existing = folder / "old.json"
            existing.write_text("原有主题", encoding="utf-8")
            with (
                patch.object(生成主题, "THEMES", folder),
                patch.object(生成主题, "生成主题对象", side_effect=ValueError("生成失败")),
            ):
                with self.assertRaises(ValueError):
                    生成主题.写入主题(生成主题.构建计划(["blue"]))
            self.assertEqual(existing.read_text(encoding="utf-8"), "原有主题")


class ExtensionStatusTest(unittest.TestCase):
    def test_missing_development_tools_fail(self) -> None:
        for action, missing in (
            ("类型检查", "ty、tsc、node"),
            ("静态检查", "ruff"),
            ("格式化", "ruff"),
        ):
            with self.subTest(action=action):
                result = subprocess.run(
                    [sys.executable, str(SRC / "开发检查.py"), action],
                    env={"PATH": ""},
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"缺少开发工具：{missing}", result.stdout)

    def test_vsix_content_types(self) -> None:
        import 打包扩展

        root = ET.fromstring(打包扩展.CONTENT_TYPES)
        entries = {entry.attrib["Extension"]: entry.attrib["ContentType"] for entry in root}
        self.assertTrue(all(not extension.startswith(".") for extension in entries))
        self.assertEqual(entries["css"], "text/css")
        self.assertEqual(entries["json"], "application/json")

    def test_vsix_xml_escapes_metadata(self) -> None:
        import 打包扩展

        manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        manifest["description"] = '说明 <示例> & "引号"'
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
            icon = folder / manifest["icon"]
            icon.parent.mkdir(parents=True)
            icon.write_bytes((ROOT / manifest["icon"]).read_bytes())
            changelog = folder / "generated.md"
            changelog.write_text("# 自动生成日志\n", encoding="utf-8")
            with patch.object(打包扩展, "ROOT", folder):
                打包扩展.入口(changelog)
            with zipfile.ZipFile(
                folder / f"{manifest['name']}-{manifest['version']}.vsix"
            ) as archive:
                xml = ET.fromstring(archive.read("extension.vsixmanifest"))
                description = xml.find("{*}Metadata/{*}Description")
                assert description is not None
                self.assertEqual(description.text, manifest["description"])
                icon_metadata = xml.find("{*}Metadata/{*}Icon")
                assert icon_metadata is not None
                icon_uri = "extension/" + manifest["icon"]
                self.assertEqual(icon_metadata.text, icon_uri)
                assets = xml.findall("{*}Assets/{*}Asset")
                registered_icons = [
                    asset
                    for asset in assets
                    if asset.attrib["Type"] == "Microsoft.VisualStudio.Services.Icons.Default"
                ]
                self.assertEqual(len(registered_icons), 1)
                self.assertEqual(registered_icons[0].attrib["Path"], icon_uri)
                self.assertEqual(archive.read(icon_uri), icon.read_bytes())
                self.assertEqual(archive.read("extension/CHANGELOG.md"), changelog.read_bytes())
            # 配置了图标却未纳入包，或引用仓库之外的文件时必须失败。
            for bad_icon in ("missing.png", "../outside.png", str(icon)):
                manifest["icon"] = bad_icon
                (folder / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
                with patch.object(打包扩展, "ROOT", folder), self.assertRaises(ValueError):
                    打包扩展.入口(changelog)

    def test_recommended_settings_recovery(self) -> None:
        node = shutil.which("node")
        if node is None:
            self.skipTest("未安装 Node.js")
        result = subprocess.run(
            [node, str(ROOT / "测试/验证设置.cjs")],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_offline_status_panel(self) -> None:
        node = shutil.which("node")
        if node is None:
            self.skipTest("未安装 Node.js，跳过扩展状态面板测试")
        result = subprocess.run(
            [node, str(ROOT / "测试" / "验证状态.cjs")],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class CssTest(unittest.TestCase):
    def test_missing_vscode_css_fails(self) -> None:
        import 检查样式

        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            self.assertEqual(检查样式.校验(Path(directory) / "missing.css"), 1)

    def test_css_files_parse(self) -> None:
        import 检查样式

        sheets = sorted((ROOT / "附加外观").glob("*.css"))
        self.assertTrue(sheets)
        for sheet in sheets:
            selectors, declarations = 检查样式.选择器与声明(sheet.read_text(encoding="utf-8"))
            self.assertTrue(selectors.strip(), sheet.name)
            self.assertTrue(declarations.strip(), sheet.name)

    def test_referenced_variables_are_defined(self) -> None:
        import 检查样式

        vscode_css, _ = 检查样式.查找VSCode资源(None)
        vscode_defined: set[str] = 检查样式.主题变量()
        if vscode_css is not None:
            vscode_defined |= set(
                检查样式.VAR_DEF_RE.findall(vscode_css.read_text(encoding="utf-8", errors="ignore"))
            )
        for sheet in sorted((ROOT / "附加外观").glob("*.css")):
            text = sheet.read_text(encoding="utf-8")
            _, declarations = 检查样式.选择器与声明(text)
            referenced = set(检查样式.VAR_REF_RE.findall(declarations))
            defined = set(检查样式.VAR_DEF_RE.findall(text))
            missing = sorted(referenced - defined - vscode_defined)
            self.assertFalse(missing, f"{sheet.name}: 未定义的变量 {missing}")

    def test_vscode_selectors_still_exist(self) -> None:
        import 检查样式

        vscode_css, _ = 检查样式.查找VSCode资源(None)
        if vscode_css is None:
            self.skipTest("未找到已安装的 VS Code")
        self.assertEqual(检查样式.校验(None), 0)


class ProductIconSourcesTests(unittest.TestCase):
    def test_imported_sources_match_assets_and_mappings(self) -> None:
        """上游资产保持原字节；码点、字体与所有别名必须能对应到来源记录。"""
        folder = ROOT / "产品图标"
        sources = json.loads((folder / "来源.json").read_text(encoding="utf-8"))
        theme = json.loads((folder / "adwcode.json").read_text(encoding="utf-8"))
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
                self.assertEqual(
                    hashlib.sha256(path.read_bytes()).hexdigest(), entry["sha256"], entry["file"]
                )
                self.assertEqual(Path(entry["upstream_path"]).name, path.name)
                self.assertEqual(ET.parse(path).getroot().tag, "{http://www.w3.org/2000/svg}svg")
                preview = folder / "渲染图标" / font["id"] / (entry["codepoint"] + ".svg")
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
                    self.assertEqual(
                        theme["iconDefinitions"][icon],
                        {
                            "fontId": font["id"],
                            "fontCharacter": "\\" + entry["codepoint"],
                        },
                    )
        self.assertEqual(
            mapped,
            {
                icon
                for icon, definition in theme["iconDefinitions"].items()
                if definition["fontId"] in imported_ids
            },
        )

    def test_adwaita_equivalents_take_priority(self) -> None:
        """官方已有对应字形时采用 Adwaita；备用 MoreWaita 不参与运行时加载。"""
        folder = ROOT / "产品图标"
        sources = json.loads((folder / "来源.json").read_text(encoding="utf-8"))
        theme = json.loads((folder / "adwcode.json").read_text(encoding="utf-8"))
        priority = ["adwcode-adwaita", "adwcode-builder", "adwcode-morewaita"]
        self.assertEqual(sources["source_priority"], priority)
        self.assertEqual([font["id"] for font in sources["fonts"]], priority)
        for icon in (
            "terminal",
            "extensions",
            "debug-continue",
            "debug-stop",
            "package",
            "tools",
            "root-folder",
            "chrome-close",
            "chrome-maximize",
            "chrome-minimize",
            "chrome-restore",
        ):
            self.assertEqual(theme["iconDefinitions"][icon]["fontId"], "adwcode-adwaita", icon)
        self.assertNotIn("adwcode-morewaita", {font["id"] for font in theme["fonts"]})


class 中文入口测试(unittest.TestCase):
    def test_新校验参数及旧参数拒绝(self) -> None:
        for flag, expected in (("--校验", 0), ("--check", 2)):
            with self.subTest(flag=flag):
                result = subprocess.run(
                    [sys.executable, str(SRC / "生成主题.py"), flag],
                    cwd=ROOT,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, expected, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
