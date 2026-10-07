// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// @ts-check
/** @param {{ vscode: typeof import("vscode"), fs: typeof import("fs"), os: typeof import("os"), path: typeof import("path"), execFile: typeof import("child_process").execFile, 环境: Record<string, string | undefined> }} 依赖 */
function 创建外观服务(依赖) {
  const { vscode, fs, os, path, execFile, 环境 } = 依赖;
  const 组件 = require("./组件").创建组件(os, path, 环境);
  const 字体 = require("./字体").创建字体(vscode, execFile, fs, path);
  const 引用 = require("./引用").创建引用(vscode, path, 组件);
  const 状态 = require("./状态").创建状态(vscode, fs, path, 组件, 字体, 引用);
  const 事务 = require("./事务").创建事务(vscode, fs, path, 组件, 字体, 引用, 状态);
  return { ...组件, ...字体, ...引用, ...状态, ...事务 };
}
module.exports = { 创建外观服务 };
