#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""启动独立 VS Code 窗口，比较外观 CSS 开关；不安装加载器、不执行重载。"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def 入口() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--浏览器工具",
        dest="playwright",
        required=True,
        type=Path,
        help="独立安装的 playwright 模块目录",
    )
    parser.add_argument("--编辑器程序", dest="code", type=Path, help="VS Code 的 bin/code 路径")
    parser.add_argument("--次数", dest="runs", type=int, default=6, help="每个场景每种样式的样本数")
    parser.add_argument(
        "--输出", dest="output", type=Path, default=ROOT / "builddir/performance-ui.json"
    )
    parser.add_argument(
        "--仅选择器",
        dest="selectors_only",
        action="store_true",
        help="仅对比非活动状态选择器的开销",
    )
    parser.add_argument(
        "--参考样式", dest="reference_css", type=Path, help="额外比较修改前的外观 CSS"
    )
    parser.add_argument(
        "--场景",
        dest="scenario",
        choices=["all", "scroll"],
        default="all",
        help="单独复核滚动或运行全部场景",
    )
    args = parser.parse_args()
    if args.runs < 4:
        parser.error("--次数 至少为 4")
    if args.selectors_only and args.scenario != "all":
        parser.error("--仅选择器 不能与单独滚动场景同时使用")
    code_cli = args.code or Path(shutil.which("code") or "")
    code = code_cli.resolve().parent.parent / "code"
    node = shutil.which("node")
    if not code.is_file() or not node or not args.playwright.is_dir():
        parser.error("需要本机 Linux VS Code、Node.js 与独立安装的 playwright")
    args.output = args.output.resolve()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="adwcode-ui-performance-") as directory:
        folder = Path(directory)
        profile = folder / "profile"
        extensions = folder / "extensions"
        fixture = folder / "workspace"
        (profile / "User").mkdir(parents=True)
        fixture.mkdir()
        manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        extension = extensions / f"{manifest['publisher']}.{manifest['name']}-{manifest['version']}"
        extension.mkdir(parents=True)
        for name in ["扩展", "主题", "附加外观", "产品图标", "资产"]:
            shutil.copytree(ROOT / name, extension / name)
        shutil.copy2(ROOT / "package.json", extension / "package.json")
        settings = {
            "workbench.colorTheme": "AdwCode 浅色",
            "workbench.productIconTheme": "adwcode",
            "workbench.startupEditor": "none",
            "window.restoreWindows": "none",
            "adwcode.自动重载": False,
            "security.workspace.trust.enabled": False,
            "telemetry.telemetryLevel": "off",
            "update.mode": "none",
            "extensions.autoCheckUpdates": False,
            "extensions.autoUpdate": False,
            "editor.minimap.enabled": False,
            "editor.stickyScroll.enabled": False,
            "editor.smoothScrolling": False,
            "workbench.iconTheme": None,
            "window.commandCenter": False,
        }
        (profile / "User/settings.json").write_text(json.dumps(settings), encoding="utf-8")
        (fixture / "sample.py").write_text(
            "".join(
                f"def sample_{i}(value: int) -> int:\n    return value + {i}\n\n"
                for i in range(2000)
            ),
            encoding="utf-8",
        )
        for i in range(200):
            (fixture / f"file_{i:03}.txt").write_text(f"文件 {i}\n", encoding="utf-8")
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        env = {
            key: value
            for key, value in os.environ.items()
            if key
            not in [
                "ELECTRON_RUN_AS_NODE",
                "VSCODE_IPC_HOOK_CLI",
                "VSCODE_NLS_CONFIG",
                "VSCODE_CWD",
            ]
        }
        with (folder / "launch.log").open("w", encoding="utf-8") as log:
            process = subprocess.Popen(
                [
                    str(code),
                    "--user-data-dir",
                    str(profile),
                    "--extensions-dir",
                    str(extensions),
                    "--new-window",
                    "--skip-add-to-recently-opened",
                    "--locale=en",
                    "--remote-debugging-address=127.0.0.1",
                    f"--remote-debugging-port={port}",
                    "--disable-renderer-backgrounding",
                    "--disable-background-timer-throttling",
                    "--disable-backgrounding-occluded-windows",
                    str(fixture),
                    str(fixture / "sample.py"),
                ],
                env=env,
                stdout=log,
                stderr=log,
                start_new_session=True,
            )
            state = {
                "directory": str(folder),
                "port": port,
                "pid": process.pid,
                "profile": str(profile),
                "fixture": str(fixture),
                "referenceCss": str(args.reference_css.resolve()) if args.reference_css else None,
                "scenario": args.scenario,
            }
            session = folder / "session.json"
            session.write_text(json.dumps(state), encoding="utf-8")
            try:
                subprocess.run(
                    [
                        node,
                        str(ROOT / "基准/工作台基准.cjs"),
                        str(session),
                        str(args.playwright.resolve()),
                        str(args.output),
                        str(args.runs),
                        str(args.selectors_only),
                    ],
                    check=True,
                    timeout=240,
                )
            finally:
                # 只终止本次创建的独立进程组；不连接操作者的窗口。
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()


if __name__ == "__main__":
    入口()
