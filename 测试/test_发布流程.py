#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode 贡献者
"""用本地命令替身验证重复发布与上传失败，不访问 GitHub。"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from Git测试仓库 import 执行Git

ROOT = Path(__file__).resolve().parent.parent


class ReleaseWorkflowTest(unittest.TestCase):
    def test_upload_preserves_existing_release(self) -> None:
        manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        version = manifest["version"]
        asset_name = f"{manifest['name']}-{version}.vsix"
        workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        step = workflow.split("      - name: 上传到 Release\n", 1)[1].split(
            "      - name: 发布到扩展市场\n", 1
        )[0]
        lines = step.split("        run: |\n", 1)[1].splitlines()
        script = "\n".join(line[10:] for line in lines if line.startswith("          "))
        script = script.replace("${{ steps.python.outputs.python-path }}", sys.executable)
        for existing, asset, fail in (
            (True, True, False),
            (True, False, True),
            (True, False, False),
            (False, False, False),
        ):
            with (
                self.subTest(existing=existing, asset=asset, fail=fail),
                tempfile.TemporaryDirectory() as directory,
            ):
                folder = Path(directory)
                (folder / "源码").mkdir()
                shutil.copy2(ROOT / "源码/生成变更日志.py", folder / "源码/生成变更日志.py")
                shutil.copy2(ROOT / "package.json", folder / "package.json")
                执行Git(folder, "init", "--quiet")
                执行Git(folder, "add", "package.json")
                执行Git(folder, "commit", "--quiet", "-m", "新增: 测试发布")
                执行Git(folder, "tag", f"v{version}")
                (folder / asset_name).write_bytes("测试归档".encode())
                marker = folder / "existing-release"
                if existing:
                    marker.touch()
                asset_marker = folder / "existing-asset"
                if asset:
                    asset_marker.write_text("原有附件", encoding="utf-8")
                binaries = folder / "bin"
                binaries.mkdir()
                gh = binaries / "gh"
                gh.write_text(
                    '#!/bin/sh\nprintf "%s\\n" "$*" >> "$TRACE"\n'
                    'case "$2" in\n'
                    'view) test -e "$RELEASE_MARKER" || exit 1\n'
                    '  if [ "$4" = --json ] && [ -e "$ASSET_MARKER" ]; then\n'
                    '    printf "%s\\n" "$ASSET_NAME"\n'
                    "  fi ;;\n"
                    'delete) rm -f "$RELEASE_MARKER" ;;\n'
                    'upload) test "$FAIL_UPLOAD" = 0 || exit 1; touch "$ASSET_MARKER" ;;\n'
                    'download) cp "$ASSET_MARKER" "$ASSET_NAME" ;;\n'
                    'create) touch "$RELEASE_MARKER" "$ASSET_MARKER" ;;\n'
                    'edit) touch "$RELEASE_MARKER" ;;\n'
                    "*) exit 2 ;;\nesac\n",
                    encoding="utf-8",
                )
                gh.chmod(0o755)
                trace = folder / "commands.log"
                environment = {
                    **os.environ,
                    "PATH": str(binaries) + os.pathsep + os.environ.get("PATH", ""),
                    "TRACE": str(trace),
                    "RELEASE_MARKER": str(marker),
                    "ASSET_MARKER": str(asset_marker),
                    "ASSET_NAME": asset_name,
                    "FAIL_UPLOAD": "1" if fail else "0",
                    "GITHUB_REF_NAME": f"v{version}",
                    "GITHUB_REPOSITORY": "测试/仓库",
                }
                result = subprocess.run(
                    ["bash", "-e", "-o", "pipefail", "-c", script],
                    cwd=folder,
                    env=environment,
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                commands = trace.read_text(encoding="utf-8")
                self.assertNotIn("release delete", commands)
                self.assertFalse(
                    any(
                        line.startswith("release upload") and "--clobber" in line
                        for line in commands.splitlines()
                    )
                )
                self.assertTrue(marker.exists(), "上传失败不得删除现有 Release")
                if asset:
                    self.assertNotIn("release upload", commands)
                    self.assertIn("release download", commands)
                    self.assertEqual(asset_marker.read_text(encoding="utf-8"), "原有附件")
                    self.assertEqual((folder / asset_name).read_text(encoding="utf-8"), "原有附件")
                if fail:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertNotIn("release edit", commands)
                else:
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertTrue(asset_marker.exists())
                    self.assertIn("release edit" if existing else "release create", commands)


if __name__ == "__main__":
    unittest.main()
