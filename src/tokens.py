# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""基于 GNOME Builder 的 GtkSourceView 样式方案的语法高亮。

下面的 ``MAP`` 派生自 piousdeer/vscode-adwaita
（https://github.com/piousdeer/vscode-adwaita，GPL-3.0-only），并适配到
``gtksourceview_xml/`` 中存放的当前 GtkSourceView 5 方案：

- ``def:keyword`` 更名为 ``def:statement``；两个名字都接受。
- 新增 ``def:emphasis``、``def:identifier``、``def:inline-code``、
  ``def:note`` 与 ``def:deletion``。
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Literal, TypedDict
from xml.etree.ElementTree import parse

from palette import to_hex


class StyleInfo(TypedDict):
    """GtkSourceView ``<style>`` 元素解析后的颜色与字体样式。"""

    foreground: str | None
    background: str | None
    fontStyle: str


class Scheme(TypedDict):
    """解析后的 GtkSourceView 方案。"""

    named: dict[str, str | None]
    styles: dict[str, StyleInfo]


class TokenRule(TypedDict):
    """TextMate 规则：Builder 生成，或随附的 VS Code 默认主题。

    后者常把 ``scope`` 写成单个字符串，因此这里同时接受两种形态。
    """

    scope: str | list[str]
    settings: dict[str, str]


XML_DIR: Path = Path(__file__).parent / "gtksourceview_xml"

#: GtkSourceView 样式名 -> TextMate 作用域。
MAP: dict[str, list[str]] = {
    # 默认颜色
    "text": [
        # 空选择器作用于所有内容
        "",
        # 内嵌表达式（例如字符串中的 ${→something←}）
        "meta.embedded",
        # 显式使用默认色的变量
        "variable",
        # XML 属性中的内嵌表达式标点
        "meta.tag.attributes punctuation.section.embedded",
        # 大多数运算符为符号型
        "keyword.operator",
        "storage.type.function.arrow",  # =>
        # YAML 符号关键字
        "keyword.control.flow.block-scalar.literal",
        "keyword.control.flow.block-scalar.folded",
        "storage.modifier.chomping-indicator",
        # 字符串前缀（例如 Python 的 f、b、r）
        "storage.type.string",
        "string.quoted.byte.raw",
        # Rust
        "meta.macro.rules entity.name.function.macro.rust",
    ],
    "def:base-n-integer": [
        "constant.numeric.binary",
        "constant.numeric.octal",
        "constant.numeric.hex",
        "keyword.other.unit.binary",
        "keyword.other.unit.octal",
        "keyword.other.unit.hexadecimal",
        "keyword.other.unit.imaginary",
        "keyword.other.unit.exponent",
    ],
    "def:boolean": [
        "constant.language.boolean",
        "constant.language.bool",
    ],
    "def:comment": [
        "comment",
        # YAML
        "entity.other.document.begin.yaml",
        "entity.other.document.end.yaml",
    ],
    "def:constant": [
        "constant.language",
        # 字符（例如 Rust 中）
        "string.quoted.single.char",
        # { →"key"←: ... }（例如 JSON 中）
        "support.type.property-name",
        # CSS
        "support.constant.property-value.css",
        "source.css keyword.other.unit",
    ],
    "def:decimal": [
        "constant.numeric",
        "constant.numeric entity.name.type.numeric",  # 1→i64← (in e.g. Rust)
    ],
    "def:deletion": [
        "markup.strikethrough",
        "markup.strikethrough.markdown",
    ],
    "def:doc-comment-element": [
        "comment.block.documentation",
    ],
    "def:emphasis": [
        "markup.italic",
        "markup.italic.markdown",
    ],
    "def:floating-point": [
        "constant.numeric.float",
    ],
    # 注意：gtksv 对 def:function 的应用并不一致（仅 Python 定义处生效），
    # 因此函数保持默认色，与上游一致。
    "def:function": [],
    "def:heading": [
        "markup.heading.markdown",
    ],
    "def:identifier": [
        "variable.other",
        "variable.other.readwrite",
        "variable.other.object",
        "meta.definition.variable",
        "support.variable",
    ],
    "def:inline-code": [
        "markup.inline.raw",
        "markup.raw.inline",
        "markup.inline.raw.string.markdown",
    ],
    "def:statement": [
        # 大多数关键字（运算符在 `text` 中不着色）
        "keyword",
        # 字母型运算符与关键字
        "keyword.operator.new",
        "keyword.operator.logical.python",
        "source.js keyword.operator.expression",
        "source.ts keyword.operator.expression",
        "storage.modifier",
        "storage.type.class",
        "storage.type.function",
        # YAML 键名属于标签名
        "entity.name.tag.yaml",
        # 针对用 `storage.type` 标记关键字的扩展的兼容处理
        "source.js storage.type",
        "source.ts storage.type",
        "source.tsx storage.type",
        "source.rust storage.type",
    ],
    # 为旧版 GtkSourceView 方案保留的别名。
    "def:keyword": [
        "keyword",
        "keyword.operator.new",
        "keyword.operator.logical.python",
        "source.js keyword.operator.expression",
        "source.ts keyword.operator.expression",
        "storage.modifier",
        "storage.type.class",
        "storage.type.function",
        "entity.name.tag.yaml",
        "source.js storage.type",
        "source.ts storage.type",
        "source.tsx storage.type",
        "source.rust storage.type",
    ],
    "def:number": [
        "constant.numeric",
    ],
    "def:note": [
        "keyword.codetag.notation",
        "comment.line.note",
    ],
    "def:preprocessor": [
        "meta.preprocessor",
        "meta.preprocessor keyword.control",
        "punctuation.decorator",
        "meta.decorator entity.name.function",
        "entity.name.function.decorator",
        "keyword.control.at-rule.media",
        "constant.character.entity",
        "punctuation.section.embedded",
        "punctuation.definition.template-expression",
    ],
    "def:shebang": [
        "comment.line.number-sign.shebang",
    ],
    "def:special-char": [
        "constant.character.escape",
    ],
    "def:string": [
        "string",
    ],
    "def:strong-emphasis": [
        "markup.bold.markdown",
    ],
    "def:type": [
        "storage.type",
        "entity.name.type",
        "entity.name.namespace",
        "keyword.type.cs",
        "support.type",
        "support.class.builtin",
        "support.class.promise",
    ],
    "def:underlined": [],
    "def:warning": [],
    # C#
    "c-sharp:format": [],
    "c-sharp:preprocessor": [
        "meta.preprocessor.cs",
    ],
    # C
    "c:printf": [
        "constant.other.placeholder",
    ],
    "c:signal-name": [],
    "c:storage-class": [
        "source.c storage.modifier",
    ],
    "c:type-keyword": [
        "source.c storage.type",
    ],
    # CSS
    "css:id-selector": [
        "entity.other.attribute-name.id.css",
    ],
    "css:property-name": [
        "support.type.property-name.css",
    ],
    "css:pseudo-selector": [
        "entity.other.attribute-name.pseudo-element.css",
        "entity.other.attribute-name.pseudo-class.css",
        "meta.selector.css punctuation.section.function",
    ],
    "css:selector-symbol": [
        "meta.selector.css keyword.operator",
        "entity.other.attribute-name.css",
    ],
    "css:type-selector": [],
    "css:vendor-specific": [
        "support.type.vendored.property-name.css",
    ],
    # Diff
    "diff:added-line": [
        "markup.inserted.diff",
    ],
    "diff:changed-line": [
        "markup.changed",
    ],
    "diff:diff-file": [
        "meta.diff.header",
    ],
    "diff:location": [
        "meta.diff.range",
    ],
    "diff:removed-line": [
        "markup.deleted.diff",
    ],
    # Go
    "go:printf": [
        "constant.other.placeholder.go",
    ],
    # Python
    "python:builtin-function": [
        "support.function.builtin.python",
    ],
    "python:class-name": [
        "entity.name.type.class.python",
    ],
    "python:module-handler": [
        "keyword.control.import.python",
    ],
    # Rust
    "rust:attribute": [
        "meta.attribute.rust",
        "meta.attribute.rust keyword.operator",
    ],
    "rust:lifetime": [
        "entity.name.type.lifetime.rust",
    ],
    "rust:macro": [
        "entity.name.function.macro",
    ],
    # XML
    "xml:attribute-name": [
        "meta.tag entity.other.attribute-name",
        "meta.tag keyword.operator.assignment",
        "punctuation.separator.key-value.html",
        "punctuation.separator.key-value.svelte",
        "text.xml meta.tag",
    ],
    "xml:attribute-value": [
        "meta.tag string",
    ],
    "xml:element-name": [
        "entity.name.tag",
        "support.class.component.svelte",
        "punctuation.definition.tag",
    ],
    "xml:namespace": [],
    "xml:processing-instruction": [
        "text.xml meta.tag.preprocessor entity.name.tag",
        "text.xml meta.tag.preprocessor punctuation.definition.tag",
    ],
}

#: 预期在当前方案中不存在的样式。
TOLERATED_MISSING: set[str] = {"def:keyword", "c-sharp:format", "diff:changed-line"}

_RGBA_RE: re.Pattern[str] = re.compile(r"#rgba\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)")


def load_scheme(mode: str) -> Scheme:
    """解析随附的 GtkSourceView 方案并解析其中的具名颜色。"""
    path = XML_DIR / ("Adwaita-dark.xml" if mode == "dark" else "Adwaita.xml")
    root = parse(path).getroot()
    named: dict[str, str | None] = {}
    for color in root.findall("color"):
        name = color.get("name")
        if name is not None:
            named[name] = color.get("value")

    def resolve(value: str | None) -> str | None:
        if value is None:
            return None
        match = _RGBA_RE.fullmatch(value)
        if match:
            r, g, b, a = (float(part) for part in match.groups())
            return to_hex(int(r), int(g), int(b), a)
        if value.startswith("#"):
            return value.lower()
        if value in named:
            return resolve(named[value])
        raise KeyError(f"{path.name} 中存在未知颜色名称 {value!r}")

    styles: dict[str, StyleInfo] = {}
    for style in root.findall("style"):
        name = style.get("name")
        if name is None:
            continue
        font_styles = [
            name
            for name in ("italic", "bold", "underline", "strikethrough")
            if style.get(name) in ("true", "1")
        ]
        styles[name] = {
            "foreground": resolve(style.get("foreground")),
            "background": resolve(style.get("background")),
            "fontStyle": " ".join(font_styles),
        }
    return {"named": named, "styles": styles}


def editor_colors(mode: str) -> dict[str, str | None]:
    """供 :class:`palette.Palette` 使用的编辑器表面颜色。"""
    styles = load_scheme(mode)["styles"]

    def style(name: str, key: Literal["foreground", "background"]) -> str | None:
        info = styles.get(name)
        return info[key] if info is not None else None

    return {
        "text_bg": style("text", "background"),
        "text_fg": style("text", "foreground"),
        "current_line": style("current-line", "background"),
        "cursor": style("cursor", "foreground"),
        "line_numbers_bg": style("line-numbers", "background"),
        "line_numbers_fg": style("line-numbers", "foreground"),
        "search_match_bg": style("search-match", "background"),
        "search_match_fg": style("search-match", "foreground"),
        "background_pattern": style("background-pattern", "background"),
    }


def token_colors(mode: str) -> list[TokenRule]:
    """Builder 语法高亮的 TextMate 规则。"""
    styles = load_scheme(mode)["styles"]
    rules: list[TokenRule] = []
    for style_name, scopes in MAP.items():
        if not scopes:
            continue
        style = styles.get(style_name)
        if style is None:
            if style_name not in TOLERATED_MISSING:
                print(f"警告：样式 {style_name!r} 不在方案中（{mode}）")
            continue
        settings: dict[str, str] = {"fontStyle": style["fontStyle"]}
        if style["foreground"]:
            settings["foreground"] = style["foreground"]
        if style["background"]:
            settings["background"] = style["background"]
        rules.append({"scope": list(scopes), "settings": settings})
    return rules


def semantic_token_colors(mode: str) -> dict[str, str | None]:
    """映射到 Builder 方案颜色的语义高亮。"""
    styles = load_scheme(mode)["styles"]

    def color(name: str, fallback: str | None = None) -> str | None:
        info = styles.get(name)
        return (info.get("foreground") if info is not None else None) or fallback

    type_color = color("def:type")
    identifier = color("def:identifier")
    return {
        "namespace": identifier,
        "type": type_color,
        "class": type_color,
        "enum": type_color,
        "interface": type_color,
        "struct": type_color,
        "typeParameter": type_color,
        "function": color("def:function"),
        "method": color("def:function"),
        "property": identifier,
        "variable": identifier,
        "parameter": identifier,
        "macro": color("def:preprocessor"),
        "keyword": color("def:statement"),
        "comment": color("def:comment"),
        "string": color("def:string"),
        "number": color("def:number"),
        "regexp": color("def:special-char"),
        "operator": color("text"),
        "decorator": color("def:preprocessor"),
        "deprecated": identifier,
    }
