// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

function 判断发布(目录, 引用, 主线 = "refs/remotes/origin/main") {
    const 版本 = JSON.parse(fs.readFileSync(path.join(目录, "package.json"), "utf8")).version;
    if (typeof 版本 !== "string" || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(版本)) {
        throw new Error("扩展版本无效");
    }
    const 标签 = 引用.startsWith("refs/tags/");
    if (标签 && 引用 !== `refs/tags/v${版本}`) throw new Error("标签与扩展版本不一致");
    // 缺少主线或 Git 出错时失败，不能把不确定结果当作正式发布。
    execFileSync("git", ["rev-parse", "--verify", `${主线}^{commit}`], { cwd: 目录, stdio: "pipe" });
    let 主线包含 = false;
    try {
        execFileSync("git", ["merge-base", "--is-ancestor", "HEAD", 主线], { cwd: 目录, stdio: "pipe" });
        主线包含 = true;
    } catch (错误) {
        if (错误.status !== 1) throw 错误;
    }
    const 候选 = !主线包含 || 版本.split("+")[0].includes("-");
    return { 版本, 候选, 市场: 标签 && !候选 };
}

if (require.main === module) {
    try {
        const [目录, 引用, ...多余] = process.argv.slice(2);
        if (!目录 || !引用 || 多余.length) throw new Error("用法：node 测试/发布策略.cjs <仓库目录> <Git 引用>");
        const 策略 = 判断发布(目录, 引用);
        process.stdout.write(`版本：${策略.版本}；候选：${策略.候选}；市场：${策略.市场}\n`);
        if (process.env.GITHUB_OUTPUT) {
            fs.appendFileSync(process.env.GITHUB_OUTPUT, `候选=${策略.候选}\n市场=${策略.市场}\n`);
        }
    } catch (错误) {
        process.stderr.write(`${错误.message}\n`);
        process.exitCode = 1;
    }
}
module.exports = { 判断发布 };
