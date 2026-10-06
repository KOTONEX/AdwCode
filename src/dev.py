#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""供 Meson 调用的开发检查，固定工作目录并传播子命令失败状态。"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def typecheck() -> None:
    """检查 Python 与 JavaScript；工具缺失时失败，不静默跳过。"""
    tools = {name: shutil.which(name) for name in ("ty", "tsc", "node")}
    missing = [name for name, command in tools.items() if command is None]
    if missing:
        raise RuntimeError("缺少开发工具：" + "、".join(missing))
    subprocess.run([str(tools["ty"]), "check", "--error-on-warning"], cwd=ROOT, check=True)
    subprocess.run([str(tools["tsc"]), "-p", "tsconfig.json"], cwd=ROOT, check=True)


def ruff(*arguments: str) -> None:
    """按仓库配置检查或格式化全部 Python 文件。"""
    command = shutil.which("ruff")
    if command is None:
        raise RuntimeError("缺少开发工具：ruff")
    subprocess.run([command, *arguments, "."], cwd=ROOT, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("lint", "typecheck", "format"))
    args = parser.parse_args()
    try:
        if args.action == "format":
            ruff("format")
        else:
            if args.action == "lint":
                ruff("check", "--output-format", "concise")
                ruff("format", "--check", "--output-format", "concise")
            typecheck()
        if args.action == "lint":
            # Ruff 只处理 Python；对其余 JavaScript 文件做无副作用的语法检查。
            node = str(shutil.which("node"))
            for folder in ("extension", "extras", "tests", "benchmarks"):
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
    raise SystemExit(main())
