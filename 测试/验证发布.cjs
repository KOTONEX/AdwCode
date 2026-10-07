// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { 判断发布 } = require("./发布策略.cjs");
const 目录 = fs.mkdtempSync(path.join(os.tmpdir(), "adwcode-发布-"));
const 执行git = (...参数) => execFileSync("git", 参数, { cwd: 目录, stdio: "pipe" });
try {
    执行git("init", "-b", "main");
    执行git("config", "user.name", "离线测试");
    执行git("config", "user.email", "test@example.invalid");
    fs.writeFileSync(path.join(目录, "package.json"), JSON.stringify({ version: "4.0.0" }));
    执行git("add", ".");
    执行git("commit", "-m", "主线基线");
    执行git("update-ref", "refs/remotes/origin/main", "HEAD");
    assert.deepEqual(判断发布(目录, "refs/tags/v4.0.0"), { 版本: "4.0.0", 候选: false, 市场: true });
    assert.equal(判断发布(目录, "refs/heads/main").市场, false);
    assert.throws(() => 判断发布(目录, "refs/tags/v3.3.0"), /标签与扩展版本不一致/);
    执行git("checkout", "-b", "rust");
    执行git("commit", "--allow-empty", "-m", "分支迭代");
    assert.deepEqual(判断发布(目录, "refs/tags/v4.0.0"), { 版本: "4.0.0", 候选: true, 市场: false });
    assert.throws(() => 判断发布(目录, "refs/tags/v4.0.0", "refs/heads/不存在"));
    // 进入主线后允许正式版；带预发布版本号仍保持候选。
    执行git("update-ref", "refs/remotes/origin/main", "HEAD");
    assert.equal(判断发布(目录, "refs/tags/v4.0.0").市场, true);
    fs.writeFileSync(path.join(目录, "package.json"), JSON.stringify({ version: "4.0.0-beta.1" }));
    assert.deepEqual(判断发布(目录, "refs/tags/v4.0.0-beta.1"), { 版本: "4.0.0-beta.1", 候选: true, 市场: false });
    console.log("发布策略验证通过");
} finally {
    fs.rmSync(目录, { recursive: true, force: true });
}
