#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""在临时 Git 仓库验证版本范围、发布校验及自动日志分发。"""

from __future__ import annotations

import io
import json
import shutil
import sys
import tempfile
import unittest
import zipfile
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from Git测试仓库 import 执行Git

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import 打包扩展
import 生成变更日志


class GitReleaseNotesTests(unittest.TestCase):
    def prepare(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        执行Git(self.root, "init", "--quiet")
        self.manifest = {
            "name": "AdwCode",
            "version": "1.1.0",
            "publisher": "测试",
            "displayName": "AdwCode",
            "description": "测试归档",
            "engines": {"vscode": "^1.100.0"},
            "repository": {"url": "https://github.com/example/AdwCode.git"},
        }
        (self.root / "package.json").write_text(json.dumps(self.manifest), encoding="utf-8")
        执行Git(self.root, "add", "package.json")
        self.commit("初始化: 测试项目")

    def commit(self, title: str) -> str:
        执行Git(self.root, "commit", "--allow-empty", "--quiet", "-m", title)
        return 执行Git(self.root, "rev-parse", "HEAD")

    def test_version_ranges_and_pending_changes(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        self.commit("新增: 文件页签")
        self.commit("修复: 对齐图标")
        执行Git(self.root, "tag", "-a", "v1.1.0", "-m", "测试版本")
        self.commit("文档: 补充许可")
        (self.root / "CHANGELOG.md").write_text("不要读取手写内容", encoding="utf-8")
        log = 生成变更日志.生成变更日志(self.root)
        pending, published = log.split("## [1.1.0]", 1)
        self.assertIn("补充许可", pending)
        self.assertNotIn("文件页签", pending)
        current, initial = published.split("## [1.0.0]", 1)
        self.assertIn("### 新增", current)
        self.assertIn("文件页签", current)
        self.assertIn("对齐图标", current)
        self.assertNotIn("补充许可", current)
        self.assertIn("测试项目", initial)
        self.assertNotIn("不要读取手写内容", log)
        self.assertEqual(log, 生成变更日志.生成变更日志(self.root))

    def test_initial_release_unknown_titles_and_markdown(self) -> None:
        self.prepare()
        self.commit("旧格式提交 <script> [链接]")
        执行Git(self.root, "tag", "v1.1.0")
        notes = 生成变更日志.生成发布说明(self.root, "1.1.0", "v1.1.0")
        self.assertIn("### 初始化", notes)
        self.assertIn("### 其他", notes)
        self.assertIn(r"旧格式提交 \<script\> \[链接\]", notes)
        self.assertNotIn("未打标签预览", notes)

    def test_merge_keeps_branch_changes_but_ignores_branch_tags(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        执行Git(self.root, "checkout", "--quiet", "-b", "feature")
        self.commit("新增: 分支功能")
        执行Git(self.root, "tag", "v9.0.0")
        执行Git(self.root, "checkout", "--quiet", "main")
        self.commit("修复: 主线问题")
        执行Git(self.root, "merge", "--no-ff", "--quiet", "feature", "-m", "合并开发分支")
        执行Git(self.root, "tag", "checkpoint")
        执行Git(self.root, "tag", "v1.1.0")
        log = 生成变更日志.生成变更日志(self.root)
        self.assertIn("分支功能", log)
        self.assertIn("主线问题", log)
        self.assertNotIn("合并开发分支", log)
        self.assertNotIn("## [9.0.0]", log)
        notes = 生成变更日志.生成发布说明(self.root, "1.1.0", "v1.1.0")
        self.assertIn("/compare/v1.0.0...v1.1.0", notes)
        self.assertNotIn("checkpoint", notes)

    def test_release_rejects_wrong_tag_and_old_or_rewritten_head(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        self.commit("新增: 当前功能")
        执行Git(self.root, "tag", "v1.1.0")
        for version, tag in (("1.1.0", "v1.0.0"), ("1.0.0", "v1.0.0"), ("1.2.0", "v1.2.0")):
            with self.subTest(version=version, tag=tag), self.assertRaises(ValueError):
                生成变更日志.生成发布说明(self.root, version, tag)
        执行Git(
            self.root, "commit", "--amend", "--allow-empty", "--quiet", "-m", "新增: 覆盖本地功能"
        )
        log = 生成变更日志.生成变更日志(self.root)
        self.assertNotIn("## [1.1.0]", log)
        preview = 生成变更日志.生成发布说明(self.root, "1.1.0")
        self.assertIn("未打标签预览", preview)
        self.assertIn("覆盖本地功能", preview)
        with self.assertRaisesRegex(ValueError, "当前 HEAD"):
            生成变更日志.生成发布说明(self.root, "1.1.0", "v1.1.0")

    def test_shallow_clone_and_missing_git_fail(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        self.commit("新增: 后续功能")
        with tempfile.TemporaryDirectory() as directory:
            clone = Path(directory) / "clone"
            执行Git(self.root, "clone", "--quiet", "--depth=1", self.root.as_uri(), str(clone))
            with self.assertRaisesRegex(ValueError, "历史不完整"):
                生成变更日志.生成变更日志(clone)
        with (
            patch.object(生成变更日志.subprocess, "run", side_effect=FileNotFoundError("缺少 git")),
            self.assertRaisesRegex(ValueError, "无法运行 Git"),
        ):
            生成变更日志.生成变更日志(self.root)

    def test_failed_generation_preserves_existing_notes(self) -> None:
        self.prepare()
        output = self.root / "release.md"
        output.write_text("原有说明", encoding="utf-8")
        with (
            patch.object(生成变更日志, "ROOT", self.root),
            patch.object(
                sys,
                "argv",
                ["生成变更日志.py", "发布说明", "--标签", "v1.1.0", "--输出", str(output)],
            ),
            self.assertRaises(SystemExit),
        ):
            生成变更日志.入口()
        self.assertEqual(output.read_text(encoding="utf-8"), "原有说明")

    def test_duplicate_tags_are_rejected(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        执行Git(self.root, "tag", "v1.1.0")
        with self.assertRaisesRegex(ValueError, "多个版本标签"):
            生成变更日志.生成变更日志(self.root)

    def test_tag_name_collision_and_prerelease_versions(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        执行Git(self.root, "branch", "v1.0.0")
        self.commit("新增: 预发布功能")
        执行Git(self.root, "tag", "v1.1.0-rc.1+build.2")
        log = 生成变更日志.生成变更日志(self.root)
        self.assertIn("## [1.0.0]", log)
        self.assertIn("## [1.1.0-rc.1+build.2]", log)
        notes = 生成变更日志.生成发布说明(self.root, "1.1.0-rc.1+build.2", "v1.1.0-rc.1+build.2")
        self.assertIn("预发布功能", notes)
        for version in ("01.1.0", "1.1.0-01", "1.1.0-rc_1", "1.1.0-rc..1", "1.1.0+"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                生成变更日志.生成发布说明(self.root, version)

    def test_packaging_generates_log_and_exported_copy_requires_log(self) -> None:
        self.prepare()
        执行Git(self.root, "tag", "v1.0.0")
        self.commit("新增: 本次功能")
        (self.root / "CHANGELOG.md").write_text("不能打包这份手写日志", encoding="utf-8")
        with patch.object(打包扩展, "ROOT", self.root), redirect_stdout(io.StringIO()):
            打包扩展.入口()
        generated = 生成变更日志.生成变更日志(self.root)
        with zipfile.ZipFile(self.root / "AdwCode-1.1.0.vsix") as archive:
            self.assertEqual(archive.read("extension/CHANGELOG.md").decode(), generated)
            self.assertEqual(archive.namelist().count("extension/CHANGELOG.md"), 1)
        with tempfile.TemporaryDirectory() as directory:
            exported = Path(directory)
            shutil.copy2(self.root / "package.json", exported / "package.json")
            log = exported / "generated.md"
            log.write_text(generated, encoding="utf-8")
            with patch.object(打包扩展, "ROOT", exported), redirect_stdout(io.StringIO()):
                with self.assertRaises(ValueError):
                    打包扩展.入口()
                打包扩展.入口(log)
                log.write_text("", encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "为空"):
                    打包扩展.入口(log)


if __name__ == "__main__":
    unittest.main()
