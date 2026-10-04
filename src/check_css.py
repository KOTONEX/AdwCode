#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""对照已安装的 VS Code 检查 extras/ 中的自定义 CSS。

VS Code 更名类名或移除设计令牌时，CSS 补丁就会失效。本脚本解析项目样式表并校验：

- 每个类选择器仍存在于 VS Code 编译后的 CSS 中；
- 样式引用的每个 ``var(--vscode-*)`` 要么由 VS Code 定义、属于主题色注册表，
  要么由项目的令牌块定义。

用法：
    python3.14t src/check_css.py [--css PATH] [--verbose]
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import cast

ROOT: Path = Path(__file__).parent.parent
EXTRAS: Path = ROOT / "extras"
REGISTRY_KEYS: Path = ROOT / "src" / "vscode_defaults" / "registry_keys.json"

CSS_CANDIDATES: list[Path] = [
    # Homebrew cask 安装
    Path("/home/linuxbrew/.linuxbrew/Caskroom/visual-studio-code-linux"),
    # 系统安装
    Path("/usr/share/code/resources/app/out/vs/workbench"),
    Path("/usr/lib/code/resources/app/out/vs/workbench"),
    Path("/opt/visual-studio-code/resources/app/out/vs/workbench"),
    Path("/usr/share/code-oss/resources/app/out/vs/workbench"),
    # 用户目录安装
    Path.home() / ".local/share/code/resources/app/out/vs/workbench",
]

COMMENT_RE: re.Pattern[str] = re.compile(r"/\*.*?\*/", re.S)
BLOCK_RE: re.Pattern[str] = re.compile(r"\{[^{}]*\}", re.S)
CLASS_RE: re.Pattern[str] = re.compile(r"\.([a-zA-Z][\w-]*)")
VAR_REF_RE: re.Pattern[str] = re.compile(r"var\(\s*(--vscode-[\w-]+)")
VAR_DEF_RE: re.Pattern[str] = re.compile(r"(--vscode-[\w-]+)\s*:")


def find_vscode_assets(explicit: Path | None = None) -> tuple[Path | None, Path | None]:
    """返回已安装 VS Code 的（workbench CSS, workbench JS）。"""
    if explicit:
        if explicit.is_file():
            js = explicit.with_suffix(".js")
            return explicit, (js if js.is_file() else None)
        return None, None
    for candidate in CSS_CANDIDATES:
        matches: list[Path] = []
        if candidate.is_file() and candidate.suffix == ".css":
            matches = [candidate]
        elif candidate.is_dir():
            matches = sorted(candidate.glob("**/workbench.desktop.main.css"))
        if matches:
            css = matches[0]
            js = css.with_suffix(".js")
            return css, (js if js.is_file() else None)
    return None, None


def theme_variables() -> set[str]:
    """主题色注册表的键会被 VS Code 以 ``--vscode-<键，点换横线>`` 注入。"""
    if not REGISTRY_KEYS.is_file():
        return set()
    keys = cast(list[str], json.loads(REGISTRY_KEYS.read_text(encoding="utf-8")))
    return {"--vscode-" + key.replace(".", "-") for key in keys}


def selectors_and_declarations(text: str) -> tuple[str, str]:
    """把样式表拆分为选择器文本与声明文本。"""
    text = COMMENT_RE.sub("", text)
    selectors = BLOCK_RE.sub(" ", text)
    declarations = " ".join(match.group(0)[1:-1] for match in BLOCK_RE.finditer(text))
    return selectors, declarations


def check(css_path: Path | None, verbose: bool = False) -> int:
    vscode_css, vscode_js = find_vscode_assets(css_path)
    if vscode_css is None:
        print("check_css：未找到 VS Code 样式表，请使用 --css PATH 指定路径")
        return 0
    vscode_text = vscode_css.read_text(encoding="utf-8", errors="ignore")
    vscode_classes = set(CLASS_RE.findall(selectors_and_declarations(vscode_text)[0]))
    vscode_vars = set(VAR_DEF_RE.findall(vscode_text)) | theme_variables()
    # 由 JavaScript 创建的类名（例如窗口控制按钮）不会出现在编译后的 CSS 里，
    # 因此同时搜索 JS bundle。
    if vscode_js is not None:
        js_text = vscode_js.read_text(encoding="utf-8", errors="ignore")
        vscode_classes |= set(CLASS_RE.findall(js_text)) | set(
            re.findall(r'["\'`]([a-zA-Z][\w-]*)["\'`]', js_text)
        )

    failures = 0
    for sheet in sorted(EXTRAS.glob("*.css")):
        selectors, declarations = selectors_and_declarations(sheet.read_text())
        classes = set(CLASS_RE.findall(selectors))
        missing = sorted(classes - vscode_classes)
        if missing:
            failures += 1
            print(f"失败 {sheet.name}：{len(missing)} 个类名未出现在 {vscode_css.name} 中")
            for name in missing:
                print(f"       .{name}")
        else:
            print(f"通过 {sheet.name}：{len(classes)} 个类名仍存在于 VS Code 中")

        our_vars = set(VAR_DEF_RE.findall(sheet.read_text()))
        referenced = set(VAR_REF_RE.findall(declarations))
        undefined = sorted(referenced - vscode_vars - our_vars)
        if undefined:
            failures += 1
            print(f"失败 {sheet.name}：{len(undefined)} 个变量未定义")
            for name in undefined:
                print(f"       {name}")
        elif verbose:
            print(f"     {sheet.name}: {len(referenced)} 个变量均已定义")

    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--css", type=Path, help="workbench.desktop.main.css 的路径")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    return check(args.css, args.verbose)


if __name__ == "__main__":
    raise SystemExit(main())
