// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 测试与基准共用显式服务接口；相同工厂不共享可变实例状态。
const path = require('node:path');
const fs = require('node:fs');
const vm = require('node:vm');
function 创建测试服务(依赖) {
  return require('../扩展/外观服务').创建外观服务({
    vscode: 依赖.require('vscode'), fs: 依赖.require('fs'), os: 依赖.require('os'), path,
    execFile: 依赖.require('child_process').execFile, 环境: 依赖.process.env || {},
  });
}
// 只取真实入口的 CommonJS 导出，不访问 VM 的私有函数或变量。
function 加载测试入口(依赖) {
  const 文件 = path.join(__dirname, '../扩展/扩展.js');
  const 上下文 = { module: { exports: {} }, process: 依赖.process,
    require(name) { return name.startsWith('./') ? require(path.resolve(path.dirname(文件), name)) : 依赖.require(name); },
  };
  vm.runInNewContext(fs.readFileSync(文件, 'utf8'), 上下文, { filename: 文件 });
  return 上下文.module.exports;
}
module.exports = { 创建测试服务, 加载测试入口 };
