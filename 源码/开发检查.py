#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode 贡献者
"""供 Meson 调用的开发检查，固定工作目录并传播子命令失败状态。"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def 检查类型() -> None:
    """检查 Python 与 JavaScript；工具缺失时失败，不静默跳过。"""
    tools = {name: shutil.which(name) for name in ("ty", "tsc", "node")}
    missing = [name for name, command in tools.items() if command is None]
    if missing:
        raise RuntimeError("缺少开发工具：" + "、".join(missing))
    subprocess.run([str(tools["ty"]), "check", "--error-on-warning"], cwd=ROOT, check=True)
    subprocess.run([str(tools["tsc"]), "-p", "tsconfig.json"], cwd=ROOT, check=True)


def 调用Ruff(*arguments: str) -> None:
    """按仓库配置检查或格式化全部 Python 文件。"""
    command = shutil.which("ruff")
    if command is None:
        raise RuntimeError("缺少开发工具：ruff")
    subprocess.run([command, *arguments, "."], cwd=ROOT, check=True)


def 入口() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("静态检查", "类型检查", "格式化"))
    args = parser.parse_args()
    args.action = {"静态检查": "lint", "类型检查": "typecheck", "格式化": "format"}[args.action]
    try:
        if args.action == "format":
            调用Ruff("format")
        else:
            if args.action == "lint":
                调用Ruff("check", "--output-format", "concise")
                调用Ruff("format", "--check", "--output-format", "concise")
            检查类型()
        if args.action == "lint":
            # Ruff 只处理 Python；对其余 JavaScript 文件做无副作用的语法检查。
            node = str(shutil.which("node"))
            for folder in ("扩展", "附加外观", "测试", "基准"):
                for source in sorted((ROOT / folder).iterdir()):
                    if source.suffix in (".js", ".cjs"):
                        subprocess.run([node, "--check", str(source)], cwd=ROOT, check=True)
            json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    except (RuntimeError, subprocess.CalledProcessError, ValueError) as error:
        print(f"检查失败：{error}")
        return 1
    print(
        {
            "lint": "静态、格式与类型检查通过",
            "typecheck": "类型检查通过",
            "format": "Python 格式化完成",
        }[args.action]
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(入口())
