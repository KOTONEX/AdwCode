#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""从 microsoft/vscode 刷新颜色键表。

builtin_keys.json 保存内置主题颜色键并集，registry_keys.json 保存官方颜色标识符表，
用于主题映射校验。下载与解析全部成功后再写入文件。
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
OUT: Path = Path(__file__).parent / "VSCode默认数据"
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


def 清理JSON注释(text: str) -> str:
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


def 获取默认主题(name: str) -> dict[str, Any]:
    if name not in _cache:
        data = urllib.request.urlopen(f"{RAW}/{name}.json", timeout=30).read().decode()
        _cache[name] = json.loads(清理JSON注释(data))
    return _cache[name]


def 入口() -> None:
    # 所有下载和解析成功后再写文件，避免中途失败留下混合版本的数据。
    prepared: dict[str, str] = {}
    messages: list[str] = []
    keys: set[str] = set()
    for name in ALL_THEMES:
        keys |= set(获取默认主题(name).get("colors", {}))
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
    入口()
