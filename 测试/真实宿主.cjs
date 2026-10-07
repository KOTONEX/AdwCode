// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 仅在专用隔离窗口运行，绝不执行安装、加载器或窗口重载命令。
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');
exports.run = async function () {
  const 目录 = process.env.ADWCODE_测试目录;
  assert.ok(目录 && fs.existsSync(path.join(目录, '隔离标记')), '缺少隔离宿主标记');
  const 工作区 = vscode.workspace.workspaceFolders?.[0]?.uri;
  assert.equal(工作区?.fsPath, path.join(目录, '工作区'));
  const 扩展 = vscode.extensions.getExtension('KOTONEX.AdwCode');
  assert.ok(扩展);
  await 扩展.activate();
  const 命令 = 扩展.packageJSON.contributes.commands.map(item => item.command).sort();
  const 已注册 = await vscode.commands.getCommands(true);
  assert.ok(命令.every(id => 已注册.includes(id)));
  // get 返回配置快照，每次更新后重新获取；inspect 单独读取作用域值。
  const 配置 = () => vscode.workspace.getConfiguration('adwcode', 工作区);
  const 初始 = 配置().inspect('界面字体');
  try {
    await 配置().update('界面字体', '测试全局字体', vscode.ConfigurationTarget.Global);
    assert.equal(配置().inspect('界面字体').globalValue, '测试全局字体');
    await 配置().update('界面字体', '测试工作区字体', vscode.ConfigurationTarget.Workspace);
    assert.equal(配置().get('界面字体'), '测试工作区字体');
    assert.equal(配置().inspect('界面字体').globalValue, '测试全局字体');
    await 配置().update('界面字体', undefined, vscode.ConfigurationTarget.Workspace);
    assert.equal(配置().get('界面字体'), '测试全局字体');
    await vscode.commands.executeCommand('adwcode.查看外观安装状态');
  } finally {
    await 配置().update('界面字体', 初始?.workspaceValue, vscode.ConfigurationTarget.Workspace);
    await 配置().update('界面字体', 初始?.globalValue, vscode.ConfigurationTarget.Global);
  }
  fs.writeFileSync(path.join(目录, '结果.json'), JSON.stringify({ complete: true, version: 扩展.packageJSON.version, commands: 命令, scopes: ['Global', 'Workspace'], vscode: vscode.version }));
};
