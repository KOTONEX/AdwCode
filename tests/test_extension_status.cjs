// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
// 离线验证状态检测，不加载真实 VS Code，也不修改工作台。
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const path = require("node:path");
const files = new Map();
let imports = [];
let receive;
let disposed;
let listenerDisposed = false;
const panel = {
  webview: {
    html: "",
    onDidReceiveMessage(callback) {
      receive = callback;
      return { dispose() { listenerDisposed = true; } };
    },
  },
  onDidDispose(callback) { disposed = callback; },
};
const vscode = {
  env: { appRoot: "/app" },
  Uri: {
    file: (file) => ({ toString: () => `file://${file}` }),
    parse(value) {
      const uri = new URL(value);
      return { scheme: uri.protocol.slice(0, -1), fsPath: decodeURIComponent(uri.pathname) };
    },
  },
  workspace: { getConfiguration: () => ({ get: () => imports }) },
  extensions: { getExtension: () => ({}) },
  ViewColumn: { One: 1 },
  window: { createWebviewPanel: () => panel },
};
const sandbox = {
  require(name) {
    if (name === "vscode") return vscode;
    if (name === "os") return { homedir: () => "/user" };
    if (name === "fs") return sandbox.mockFs = {
      existsSync: (file) => files.has(file),
      readFileSync(file) {
        if (!files.has(file)) throw new Error("不可读");
        return files.get(file);
      },
    };
    return require(name);
  },
  module: { exports: {} },
  process: { platform: "linux" },
  setTimeout,
  clearTimeout,
};
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(path.join(__dirname, "../extension/extension.js"), "utf8"), sandbox);
const context = { extensionPath: "/repo" };
let rows = sandbox.appearanceStatus(context);
assert.equal(rows[0].copied, "源文件不可读");
assert.equal(rows[0].patched, "无法读取");
const html = "/app/out/vs/code/electron-browser/workbench/workbench.esm.html";
let patch = "<!-- !! VSCODE-CUSTOM-CSS-START !! -->";
for (const row of rows) {
  const content = `内容：${row.name}`;
  files.set(`/repo/extras/${row.name}`, content);
  files.set(`/user/.config/adwcode/${row.name}`, content);
  imports.push(`file:///user/.config/adwcode/${row.name}`);
  patch += row.name.endsWith(".js") ? `<script>${content}</script>` : `<style>${content}</style>`;
}
patch += "<!-- !! VSCODE-CUSTOM-CSS-END !! -->";
files.set(html, patch);
rows = sandbox.appearanceStatus(context);
assert.ok(rows.every((row) => row.copied === "已同步" && row.imported && row.patched === "磁盘补丁已更新"));
// 未包在加载器标记内的内容不能被误判为已注入。
files.set(html, patch.replace(/<!--.*?-->/g, ""));
assert.ok(sandbox.appearanceStatus(context).every((row) => row.patched === "未注入当前版本"));
files.set(html, patch);
// 开发时允许直接加载仓库源码，URI 中的非 ASCII 字符可以编码。
imports = rows.map((row) => `file:///repo/extras/${row.name}`);
assert.ok(sandbox.appearanceStatus(context).every((row) => row.imported));
imports.push("不是有效的 URI");
assert.ok(sandbox.appearanceStatus(context).every((row) => row.imported));
sandbox.showAppearanceStatus(context);
assert.ok(panel.webview.html.includes("文件与补丁均已就绪"));
files.set("/repo/extras/gnome-look.css", "新样式");
receive("refresh");
assert.ok(panel.webview.html.includes("副本待更新"));
assert.ok(panel.webview.html.includes("未注入当前版本"));
imports = [];
rows = sandbox.appearanceStatus(context);
assert.ok(rows.every((row) => !row.imported));
disposed();
assert.ok(listenerDisposed);
// 模拟对象未提供写文件、配置更新或执行命令 API；调用它们会直接失败。
console.log("外观状态：缺失、同步、过期、未配置和刷新测试通过");

// 使用模拟命令验证失败和取消路径，不向真实窗口发送重载命令。
(async () => {
  let reloads = 0;
  let errors = 0;
  let enabled = true;
  let failUpdate = true;
  sandbox.mockFs.promises = {
    async copyFile(source, target) { files.set(target, files.get(source)); },
  };
  vscode.window.showErrorMessage = () => { errors++; };
  vscode.workspace.getConfiguration = () => ({ get: () => enabled });
  vscode.commands = {
    async executeCommand(command) {
      if (command === "extension.updateCustomCSS" && failUpdate) throw new Error("补丁失败");
      if (command === "workbench.action.reloadWindow") reloads++;
    },
  };
  await sandbox.reloadWithStyles(context);
  assert.equal(errors, 1);
  assert.equal(reloads, 0);
  failUpdate = false;
  enabled = false;
  await sandbox.reloadWithStyles(context);
  assert.equal(reloads, 0);
  enabled = true;
  await sandbox.reloadWithStyles(context);
  assert.equal(reloads, 1);
  let callback;
  let cleared = false;
  sandbox.setTimeout = (fn) => { callback = fn; return 123; };
  sandbox.clearTimeout = (timer) => { cleared = timer === 123; };
  sandbox.scheduleReload(context);
  assert.equal(typeof callback, "function");
  sandbox.deactivate();
  assert.ok(cleared);
  console.log("自动重载：更新失败、设置关闭和停用清理测试通过（仅使用模拟对象）");
})().catch((error) => { console.error(error); process.exitCode = 1; });
