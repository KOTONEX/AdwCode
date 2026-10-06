#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""从当前版本更新日志生成 GitHub 发布说明。"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def extract(changelog: str, version: str) -> str:
    """只提取对应版本，缺少或空白章节时拒绝发布。"""
    pattern = rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## |\Z)"
    section = re.search(pattern, changelog, re.M | re.S)
    if section is None:
        raise ValueError("更新日志缺少当前版本的发布说明")
    content = section.group(1)
    if not isinstance(content, str) or not content.strip():
        raise ValueError("更新日志缺少当前版本的发布说明")
    return content.strip() + "\n"


def main() -> None:
    version = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))["version"]
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    try:
        notes = extract(changelog, version)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    (ROOT / "release-notes.md").write_text(notes, encoding="utf-8")


if __name__ == "__main__":
    main()
