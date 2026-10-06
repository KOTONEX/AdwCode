#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""从 microsoft/vscode 刷新 src/vscode_defaults 下的全部数据。

- dark.json / light.json：内置 “2026 Dark” / “2026 Light” 主题解析后的 `tokenColors`
  （MIT），供 “default syntax highlighting” 变体使用。
- builtin_keys.json：内置主题定义的全部颜色键并集，作为 `build.py --check`
  的覆盖度基准。
- registry_keys.json：VS Code “Theme Color” 参考文档中记录的全部颜色 id，
  用于发现映射表中的拼写错误。
"""

from __future__ import annotations

import json
import re
import urllib.request
from pathlib import Path
from typing import Any

RAW: str = (
    "https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes"
)
DOCS: str = (
    "https://raw.githubusercontent.com/microsoft/vscode-docs/main/api/references/theme-color.md"
)
OUT: Path = Path(__file__).parent / "vscode_defaults"
VARIANTS: dict[str, str] = {"dark": "2026-dark", "light": "2026-light"}
ALL_THEMES: list[str] = [
    "2026-dark",
    "2026-light",
    "dark_modern",
    "dark_plus",
    "dark_vs",
    "light_modern",
    "light_plus",
    "light_vs",
    "hc_black",
    "hc_light",
]

_cache: dict[str, dict[str, Any]] = {}


def strip_jsonc(text: str) -> str:
    out = []
    i, n, in_string, escaped = 0, len(text), False, False
    while i < n:
        char = text[i]
        if in_string:
            out.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            i += 1
            continue
        if char == '"':
            in_string = True
            out.append(char)
            i += 1
            continue
        if char == "/" and i + 1 < n and text[i + 1] == "/":
            end = text.find("\n", i)
            i = n if end < 0 else end
            continue
        if char == "/" and i + 1 < n and text[i + 1] == "*":
            end = text.find("*/", i)
            i = n if end < 0 else end + 2
            continue
        out.append(char)
        i += 1
    # 仅在字符串之外移除尾逗号，不能改写诸如 ",}" 的字符串值。
    clean = "".join(out)
    result: list[str] = []
    in_string = escaped = False
    for index, char in enumerate(clean):
        if in_string:
            result.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
        else:
            if char == '"':
                in_string = True
            if char == "," and clean[index + 1 :].lstrip().startswith(("}", "]")):
                continue
            result.append(char)
    return "".join(result)


def fetch(name: str) -> dict[str, Any]:
    if name not in _cache:
        data = urllib.request.urlopen(f"{RAW}/{name}.json", timeout=30).read().decode()
        _cache[name] = json.loads(strip_jsonc(data))
    return _cache[name]


def resolve_token_colors(name: str, seen: frozenset[str] = frozenset()) -> list[dict[str, Any]]:
    if name in seen:
        raise ValueError(f"主题 include 循环：{name}")
    seen = seen | {name}
    theme = fetch(name)
    tokens: list[dict[str, Any]] = []
    include = theme.get("include")
    if include:
        tokens += resolve_token_colors(Path(include).stem, seen)
    tokens += theme.get("tokenColors", [])
    return tokens


def main() -> None:
    # 所有下载和解析成功后再写文件，避免中途失败留下混合版本的数据。
    prepared: dict[str, str] = {}
    messages: list[str] = []
    for mode, name in VARIANTS.items():
        tokens = resolve_token_colors(name)
        prepared[f"{mode}.json"] = json.dumps({"tokenColors": tokens}, indent=2) + "\n"
        messages.append(f"{mode}: 从 {name} 提取 {len(tokens)} 条语法规则")

    keys: set[str] = set()
    for name in ALL_THEMES:
        keys |= set(fetch(name).get("colors", {}))
    prepared["builtin_keys.json"] = json.dumps(sorted(keys), indent=2) + "\n"
    messages.append(f"内置颜色键： {len(keys)}")

    markdown = urllib.request.urlopen(DOCS, timeout=60).read().decode()
    registry = set(re.findall(r"`([a-zA-Z][a-zA-Z0-9]*(?:\.[a-zA-Z0-9]+)+)`", markdown))
    registry |= set(re.findall(r"^- `([a-zA-Z][a-zA-Z0-9]*)`:", markdown, re.MULTILINE))
    registry = {
        item
        for item in registry
        if not item.startswith(("workbench.", "editor.token", "configuration.", "vscode."))
    }
    prepared["registry_keys.json"] = json.dumps(sorted(registry), indent=2) + "\n"
    messages.append(f"注册表颜色键： {len(registry)}")
    OUT.mkdir(exist_ok=True)
    for filename, content in prepared.items():
        (OUT / filename).write_text(content, encoding="utf-8")
    for message in messages:
        print(message)


if __name__ == "__main__":
    main()
