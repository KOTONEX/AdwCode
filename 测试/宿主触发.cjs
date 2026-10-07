// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
const fs = require('node:fs');
const cp = require('node:child_process');
function 需要宿主(文件, 前清单, 后清单) {
  if (文件.some(名 => /^(扩展\/|类型声明\/|测试\/|\.github\/workflows\/)/u.test(名)
    || ['tsconfig.json', '源码/库/宿主测试.rs', '源码/库/运行工具.rs', '源码/库/入口.rs', '源码/库/命令.rs'].includes(名))) return true;
  if (!文件.includes('package.json')) return false;
  const 去版本 = 值 => { const 副本 = {...值}; delete 副本.version; return JSON.stringify(副本); };
  return 去版本(前清单) !== 去版本(后清单);
}
if (require.main === module) {
  const 事件 = JSON.parse(fs.readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
  const 基线 = 事件.pull_request?.base.sha || 事件.before;
  let 相关 = true;
  if (process.env.GITHUB_EVENT_NAME !== 'workflow_dispatch' && 基线 && !/^0+$/u.test(基线)) {
    try {
      const git = 参数 => cp.execFileSync('git', 参数, {encoding:'utf8', stdio:['ignore','pipe','pipe']});
      const 文件 = git(['diff','--name-only','-z',基线,'HEAD']).split('\0').filter(Boolean);
      相关 = 需要宿主(文件, JSON.parse(git(['show',`${基线}:package.json`])), JSON.parse(fs.readFileSync('package.json','utf8')));
    } catch (错误) { console.log('无法确认改动基线，完整运行宿主测试'); }
  }
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `相关=${相关}\n`);
}
module.exports = {需要宿主};
