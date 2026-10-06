// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
// 用延迟的系统读取模拟用户切换与停用，不运行真实子进程或 VS Code 命令。
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
function fixture() {
  const values = new Map([['colorTheme', 'AdwCode 深色']]);
  const workspaceValues = new Map();
  const pending = [], updates = [], messages = [];
  let enabled = true;
  const sandbox = {
    module: { exports: {} }, process: { platform: 'linux' }, setTimeout, clearTimeout,
    require(name) {
      if (name === 'vscode') return {
        workspace: { getConfiguration(section) {
          // 每次读取返回独立快照，避免测试把旧配置对象误当成实时视图。
          const globalValues = new Map(values);
          const snapshot = new Map([...globalValues, ...workspaceValues]);
          const scope = new Map(workspaceValues);
          const autoAccent = enabled;
          return {
            get(key, fallback) { return section === 'adwcode' ? autoAccent : snapshot.get(key) ?? fallback; },
            inspect(key) { return { globalValue: globalValues.get(key), workspaceValue: scope.get(key) }; },
            async update(key, value) { updates.push([key, value]); values.set(key, value); },
          };
        } },
        ConfigurationTarget: { Global: 1 },
        window: { showInformationMessage: message => messages.push(message), showWarningMessage: message => messages.push(message) },
      };
      if (name === 'child_process') return { execFile() { throw Error('禁止真实子进程'); } };
      return require(name);
    },
    fakeContext: {},
  };
  vm.createContext(sandbox);
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../extension/extension.js'), 'utf8'), sandbox);
  vm.runInContext('extensionContext = fakeContext', sandbox);
  sandbox.startAccentMonitor = sandbox.stopAccentMonitor = () => {};
  sandbox.availableThemes = () => new Set(['AdwCode 深色', 'AdwCode 绿色 深色', 'AdwCode 红色 深色']);
  sandbox.readSystemAccent = () => new Promise(resolve => pending.push(resolve));
  return { sandbox, values, workspaceValues, pending, updates, messages, disable() { enabled = false; } };
}
async function main() {
  for (const action of ['switch', 'disable', 'deactivate']) {
    const f = fixture();
    const work = f.sandbox.syncAccent();
    assert.equal(f.pending.length, 1);
    if (action === 'switch') f.values.set('colorTheme', '其他主题');
    if (action === 'disable') f.disable();
    if (action === 'deactivate') f.sandbox.deactivate();
    f.pending[0]('green');
    await work;
    assert.equal(f.updates.length, 0, action);
  }
  const f = fixture();
  f.values.set('colorTheme', 'AdwCode 红色 深色');
  const older = f.sandbox.syncAccent();
  const newer = f.sandbox.syncAccent(true);
  f.pending[1]('red');
  await newer;
  assert.equal(f.updates.length, 0);
  assert.ok(f.messages.some(message => message.includes('已与系统强调色一致')));
  f.pending[0]('green');
  await older;
  assert.equal(f.updates.length, 0, '过期读取不得覆盖较新的结果');
  const live = fixture();
  const work = live.sandbox.syncAccent();
  live.pending[0]('green');
  await work;
  assert.deepEqual(live.updates, [['colorTheme', 'AdwCode 绿色 深色']]);
  const fallback = fixture();
  fallback.sandbox.availableThemes = () => new Set(['AdwCode 深色']);
  const missing = fallback.sandbox.syncAccent(true);
  fallback.pending[0]('green');
  await missing;
  assert.ok(fallback.messages.some(message => message.includes('未安装绿色')));
  const scoped = fixture();
  scoped.values.set('colorTheme', '其他主题');
  scoped.workspaceValues.set('colorTheme', 'AdwCode 深色');
  const scopedWork = scoped.sandbox.syncAccent(true);
  scoped.pending[0]('green');
  await scopedWork;
  assert.equal(scoped.updates.length, 0);
  assert.equal(scoped.values.get('colorTheme'), '其他主题');
  assert.ok(scoped.messages.some(message => message.includes('工作区覆盖')));
  console.log('强调色：延迟切换、关闭、停用、乱序读取与已同步提示通过');
}
main().catch(error => { console.error(error); process.exitCode = 1; });
