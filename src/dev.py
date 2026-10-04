#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""供 Meson 调用的开发检查，固定工作目录并传播子命令失败状态。"""
from __future__ import annotations

import argparse
import json
import py_compile
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("lint", "typecheck"))
    args = parser.parse_args()
    try:
        typecheck()
        if args.action == "lint":
            for folder in (ROOT / "src", ROOT / "tests"):
                for source in sorted(folder.glob("*.py")):
                    py_compile.compile(str(source), doraise=True)
            json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    except (RuntimeError, subprocess.CalledProcessError, py_compile.PyCompileError, ValueError) as error:
        print(f"检查失败：{error}")
        return 1
    print("静态与类型检查通过" if args.action == "lint" else "类型检查通过")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
