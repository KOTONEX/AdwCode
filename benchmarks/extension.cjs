// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
// 离线扩展基准：真实临时文件，模拟 VS Code API，禁止外部命令与配置写入。
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const os = require('node:os');
const { performance } = require('node:perf_hooks');
const assert = require('node:assert/strict');
const root = path.resolve(process.argv[2]);
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'adwcode-extension-performance-'));
const source = fs.readFileSync(path.join(root, 'extension/extension.js'), 'utf8');
const cssDir = path.join(temporary, '.config/adwcode');
const appRoot = path.join(temporary, 'app');
const html = path.join(appRoot, 'out/vs/code/electron-browser/workbench/workbench.esm.html');
fs.mkdirSync(cssDir, { recursive: true });
fs.mkdirSync(path.dirname(html), { recursive: true });
let writes = 0;
let processes = 0;
let commands = 0;
const disposable = () => ({ dispose() {} });
function load() {
  const panel = { webview: { html: '', onDidReceiveMessage: disposable }, onDidDispose: disposable };
  const vscode = {
    env: { appRoot }, ViewColumn: { One: 1 },
    extensions: { getExtension() { return {}; } },
    Uri: { parse(value) { const url = new URL(value); return { scheme: url.protocol.slice(0, -1), fsPath: decodeURIComponent(url.pathname) }; } },
    workspace: {
      getConfiguration(section) { return {
        get(key, fallback) { if (section === 'adwcode') return false; return fallback; },
        async update() { writes++; throw Error('禁止配置写入'); },
      }; },
      onDidChangeConfiguration: disposable,
    },
    window: { createWebviewPanel() { return panel; } },
    commands: { registerCommand: disposable, async executeCommand() { commands++; throw Error('禁止命令执行'); } },
  };
  const sandbox = { module: { exports: {} }, process: { platform: 'linux' }, setTimeout, clearTimeout,
    require(name) {
      if (name === 'vscode') return vscode;
      if (name === 'os') return { homedir: () => temporary };
      if (name === 'child_process') return { execFile() { processes++; throw Error('禁止子进程'); } };
      return require(name);
    },
  };
  vm.createContext(sandbox);
  vm.runInContext(source, sandbox);
  return { sandbox, panel, context: { extensionPath: root, subscriptions: [] } };
}
function summarize(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return { median: sorted.length % 2 ? sorted[Math.floor(sorted.length / 2)] : (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2, p95: sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1)], min: sorted[0], max: sorted.at(-1) };
}
function bench(name, prepare, invoke, count) {
  for (let i = 0; i < 10; i++) invoke(prepare());
  global.gc?.();
  const samples = [];
  for (let i = 0; i < count; i++) {
    const input = prepare();
    const start = performance.now();
    invoke(input);
    samples.push(performance.now() - start);
  }
  return { name, unit: 'ms', count, summary: summarize(samples), samples };
}
try {
  const runtime = load();
  let content = '<!-- !! VSCODE-CUSTOM-CSS-START !! -->';
  for (const name of vm.runInContext('Object.keys(CSS_FILES)', runtime.sandbox)) {
    const css = runtime.sandbox.cssSource(runtime.context, name);
    fs.writeFileSync(path.join(cssDir, name), css);
    content += name.endsWith('.js') ? `<script>${css}</script>` : `<style>${css}</style>`;
  }
  content += '<!-- !! VSCODE-CUSTOM-CSS-END !! -->';
  fs.writeFileSync(html, '<!--' + 'x'.repeat(128 * 1024) + '-->' + content);
  assert.ok(runtime.sandbox.appearanceStatus(runtime.context).every(row => row.copied === '已同步' && row.patched === '磁盘补丁已更新'));
  const results = [
    bench('脚本加载（含 VM 创建）', () => undefined, () => load(), 200),
    bench('激活与停用（自动强调色与自动重载关闭）', load, input => { input.sandbox.module.exports.activate(input.context); input.sandbox.module.exports.deactivate(); }, 200),
    bench('外观安装状态读取（真实文件）', () => runtime, input => input.sandbox.appearanceStatus(input.context), 500),
    bench('外观状态面板生成（模拟 Webview）', () => runtime, input => { input.sandbox.showAppearanceStatus(input.context); input.context.subscriptions.length = 0; }, 200),
    bench('主题名称解析（每样本 1000 次，取单次均值）', () => runtime, input => { for (let i = 0; i < 1000; i++) input.sandbox.parseTheme('AdwCode 青色 浅色 · 彩色状态栏'); }, 100),
  ];
  for (const result of results) {
    if (result.name.includes('1000')) {
      result.samples = result.samples.map(value => value / 1000);
      result.summary = summarize(result.samples);
    }
  }
  assert.equal(writes, 0);
  assert.equal(processes, 0);
  assert.equal(commands, 0);
  console.log(JSON.stringify({ node: process.version, limitations: '模拟宿主 API；激活样本关闭自动强调色与重载；不等于真实扩展宿主启动时间', results, writes, processes, commands }));
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
