#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""按 Git 版本标签和提交标题生成变更日志及发布说明。"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote

ROOT = Path(__file__).resolve().parent.parent
PRE_RELEASE = r"(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
VERSION_TAG = re.compile(
    r"v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    rf"(?:-{PRE_RELEASE}(?:\.{PRE_RELEASE})*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?",
    re.ASCII,
)
GROUPS = ("新增", "修复", "文档", "测试", "重构", "杂务", "初始化", "其他")


@dataclass(frozen=True)
class Commit:
    revision: str
    date: str
    subject: str


@dataclass(frozen=True)
class Release:
    tag: str
    revision: str


def git(root: Path, *arguments: str) -> str:
    """执行只读 Git 命令，失败时保留可诊断的错误。"""
    try:
        result = subprocess.run(
            ["git", "--no-pager", "-C", str(root), *arguments],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
    except OSError as error:
        raise ValueError(f"无法运行 Git：{error}") from error
    if result.returncode:
        raise ValueError("读取 Git 历史失败：" + result.stderr.strip())
    return result.stdout


def releases(root: Path) -> tuple[str, list[Release]]:
    """仅使用 HEAD 主线上可达的版本标签，按提交拓扑由新到旧排列。"""
    top = Path(git(root, "rev-parse", "--show-toplevel").strip()).resolve()
    if top != root.resolve():
        raise ValueError("生成变更日志需要项目自身的 Git 仓库")
    if git(root, "rev-parse", "--is-shallow-repository").strip() == "true":
        raise ValueError("Git 历史不完整，请获取完整历史和版本标签后重试")
    mainline = git(root, "rev-list", "--first-parent", "HEAD").splitlines()
    if not mainline:
        raise ValueError("Git 仓库尚无提交")
    order = {revision: index for index, revision in enumerate(mainline)}
    tags = git(root, "for-each-ref", "--merged=HEAD", "--format=%(refname:strip=2)", "refs/tags")
    result: list[Release] = []
    revisions: set[str] = set()
    for tag in tags.splitlines():
        if VERSION_TAG.fullmatch(tag) is None:
            continue
        revision = git(root, "rev-parse", f"refs/tags/{tag}^{{commit}}").strip()
        if revision not in order:
            continue
        if revision in revisions:
            raise ValueError(f"同一提交存在多个版本标签，请核对：{tag}")
        revisions.add(revision)
        result.append(Release(tag, revision))
    result.sort(key=lambda release: order[release.revision])
    return mainline[0], result


def commits(root: Path, end: str, start: str | None = None) -> list[Commit]:
    """读取版本范围内的非合并提交，包括合入主线的分支提交。"""
    revision_range = f"{start}..{end}" if start else end
    fields = git(
        root,
        "log",
        "--no-show-signature",
        "--no-merges",
        "--topo-order",
        "--reverse",
        "-z",
        "--format=%H%x00%cs%x00%s",
        revision_range,
        "--",
    ).split("\0")
    if fields[-1] == "":
        fields.pop()
    if len(fields) % 3:
        raise ValueError("Git 提交记录格式不完整")
    return [Commit(*fields[index : index + 3]) for index in range(0, len(fields), 3)]


def escape(text: str) -> str:
    """将提交标题作为普通 Markdown 文本展示。"""
    return re.sub(r"([\\`*_\[\]<>])", r"\\\1", text)


def render(entries: list[Commit]) -> str:
    """按项目中文提交类型分组，未分类标题仍完整保留。"""
    grouped: dict[str, list[str]] = {name: [] for name in GROUPS}
    for entry in entries:
        prefix, separator, description = entry.subject.partition(": ")
        group = prefix if separator and prefix in GROUPS[:-1] and description else "其他"
        title = description if group != "其他" else entry.subject
        grouped[group].append(f"- {escape(title)}（`{entry.revision[:7]}`）")
    return "\n\n".join(
        f"### {group}\n\n" + "\n".join(grouped[group]) for group in GROUPS if grouped[group]
    )


def generate_changelog(root: Path) -> str:
    """生成完整日志；未提交修改不属于 Git 提交历史。"""
    head, versions = releases(root)
    pending = commits(root, head, versions[0].revision if versions else None)
    sections = [
        "# 变更日志",
        "<!-- 由 src/release_notes.py 根据 Git 历史自动生成，请勿手工编辑。 -->",
        "记录版本范围内的非合并提交标题；未提交修改不在本日志中。",
        "## [未发布]\n\n" + (render(pending) if pending else "尚无新增提交。"),
    ]
    for index, release in enumerate(versions):
        previous = versions[index + 1].revision if index + 1 < len(versions) else None
        entries = commits(root, release.revision, previous)
        date = git(
            root, "show", "--no-show-signature", "-s", "--format=%cs", release.revision
        ).strip()
        sections.append(
            f"## [{release.tag[1:]}] - {date}\n\n"
            + (render(entries) if entries else "此范围仅包含合并提交。")
        )
    return "\n\n".join(sections) + "\n"


def generate_notes(root: Path, version: str, tag: str | None = None) -> str:
    """生成当前版本说明；发布时要求版本标签严格对应 HEAD 与清单版本。"""
    if VERSION_TAG.fullmatch("v" + version) is None:
        raise ValueError("package.json 的版本号不是有效的版本标签格式")
    head, versions = releases(root)
    current = next((release for release in versions if release.tag == "v" + version), None)
    if tag is not None:
        if tag != "v" + version:
            raise ValueError("发布标签与 package.json 的版本不一致")
        if current is None or current.revision != head:
            raise ValueError("发布标签必须存在且指向当前 HEAD")
    if versions and versions[0].revision == head:
        if versions[0].tag != "v" + version:
            raise ValueError("HEAD 的版本标签与 package.json 不一致")
        previous = versions[1] if len(versions) > 1 else None
        title = f"## [{version}]"
    else:
        previous = versions[0] if versions else None
        title = f"## [{version}]（未打标签预览）"
    entries = commits(root, head, previous.revision if previous else None)
    if not entries:
        raise ValueError("当前版本范围内没有非合并提交，无法生成发布说明")
    notes = title + "\n\n" + render(entries) + "\n"
    if tag is not None and previous is not None:
        repository = json.loads((root / "package.json").read_text(encoding="utf-8")).get(
            "repository", {}
        )
        url = repository.get("url", "") if isinstance(repository, dict) else repository
        if isinstance(url, str):
            url = url.rstrip("/").removesuffix(".git")
            if re.fullmatch(r"https://github\.com/[\w.-]+/[\w.-]+", url, re.ASCII):
                notes += (
                    f"\n**完整变更**：[{previous.tag} → {tag}]"
                    f"({url}/compare/{quote(previous.tag)}...{quote(tag)})\n"
                )
    return notes


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "action", choices=("changelog", "release"), help="生成完整日志或当前版本说明"
    )
    parser.add_argument("--tag", help="发布时校验的 v<版本> 标签，仅用于 release")
    parser.add_argument("--output", type=Path, help="输出文件，默认写入 builddir")
    args = parser.parse_args()
    if args.tag and args.action != "release":
        parser.error("--tag 仅用于 release")
    try:
        if args.action == "changelog":
            content = generate_changelog(ROOT)
            output = args.output or ROOT / "builddir/CHANGELOG.md"
        else:
            version = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))["version"]
            content = generate_notes(ROOT, version, args.tag)
            output = args.output or ROOT / "builddir/release-notes.md"
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(content, encoding="utf-8")
    except (ValueError, OSError) as error:
        raise SystemExit(str(error)) from error
    print(f"已生成 {output}")


if __name__ == "__main__":
    main()
