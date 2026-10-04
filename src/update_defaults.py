#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
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

RAW: str = "https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes"
DOCS: str = "https://raw.githubusercontent.com/microsoft/vscode-docs/main/api/references/theme-color.md"
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
    return re.sub(r",(\s*[}\]])", r"\1", "".join(out))


def fetch(name: str) -> dict[str, Any]:
    if name not in _cache:
        data = urllib.request.urlopen(f"{RAW}/{name}.json", timeout=30).read().decode()
        _cache[name] = json.loads(strip_jsonc(data))
    return _cache[name]


def resolve_token_colors(name: str) -> list[dict[str, Any]]:
    theme = fetch(name)
    tokens: list[dict[str, Any]] = []
    include = theme.get("include")
    if include:
        tokens += resolve_token_colors(Path(include).stem)
    tokens += theme.get("tokenColors", [])
    return tokens


def main() -> None:
    OUT.mkdir(exist_ok=True)
    for mode, name in VARIANTS.items():
        tokens = resolve_token_colors(name)
        (OUT / f"{mode}.json").write_text(json.dumps({"tokenColors": tokens}, indent=2) + "\n")
        print(f"{mode}: 从 {name} 提取 {len(tokens)} 条语法规则")

    keys: set[str] = set()
    for name in ALL_THEMES:
        keys |= set(fetch(name).get("colors", {}))
    (OUT / "builtin_keys.json").write_text(json.dumps(sorted(keys), indent=2) + "\n")
    print(f"内置颜色键： {len(keys)}")

    markdown = urllib.request.urlopen(DOCS, timeout=60).read().decode()
    registry = set(re.findall(r"`([a-zA-Z][a-zA-Z0-9]*(?:\.[a-zA-Z0-9]+)+)`", markdown))
    registry |= set(re.findall(r"^- `([a-zA-Z][a-zA-Z0-9]*)`:", markdown, re.MULTILINE))
    registry = {
        item
        for item in registry
        if not item.startswith(("workbench.", "editor.token", "configuration.", "vscode."))
    }
    (OUT / "registry_keys.json").write_text(json.dumps(sorted(registry), indent=2) + "\n")
    print(f"注册表颜色键： {len(registry)}")


if __name__ == "__main__":
    main()
