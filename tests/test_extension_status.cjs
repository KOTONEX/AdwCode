// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
// 离线验证状态检测，不加载真实 VS Code，也不修改工作台。
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const path = require("node:path");
async function main() {
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
  workspace: { getConfiguration: () => ({ get: (key, fallback) => key === "imports" ? imports : fallback }) },
  extensions: { getExtension: () => ({}) },
  ViewColumn: { One: 1 },
  window: { createWebviewPanel: () => panel },
};
const sandbox = {
  require(name) {
    if (name === "vscode") return vscode;
    if (name === "child_process") return { execFile(_file, args, _options, callback) { if (callback) callback(null, args.includes("monospace-font-name") ? "'等距更纱黑体 SC 11'" : "'更纱黑体 UI SC 11'"); } };
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
  files.set(`/repo/extras/${row.name}`, `内容：${row.name}`);
  const content = sandbox.cssSource(context, row.name);
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
// 源文件和副本同时引用时，不能宣称安装已经就绪。
imports.push("file:///user/.config/adwcode/gnome-look.css");
assert.equal(sandbox.appearanceStatus(context)[0].importCount, 2);
sandbox.showAppearanceStatus(context);
assert.ok(panel.webview.html.includes("重复引用（2 项）"));
assert.ok(!panel.webview.html.includes("文件与补丁均已就绪"));
files.set(html, patch.replace("<!-- !! VSCODE-CUSTOM-CSS-END !! -->", "<style>内容：gnome-look.css</style><!-- !! VSCODE-CUSTOM-CSS-END !! -->"));
assert.equal(sandbox.appearanceStatus(context)[0].patched, "重复注入，补丁待更新");
files.set(html, patch);
imports.pop();
sandbox.showAppearanceStatus(context);
assert.ok(panel.webview.html.includes("文件与补丁均已就绪"));
files.set("/repo/extras/gnome-look.css", "新样式");
await receive("refresh");
assert.ok(panel.webview.html.includes("副本待更新"));
assert.ok(panel.webview.html.includes("状态已刷新。"));
assert.ok(panel.webview.html.includes("button.focus();"));
assert.ok(!panel.webview.html.includes("<table>"));
assert.ok(panel.webview.html.includes("未注入当前版本"));
imports = [];
rows = sandbox.appearanceStatus(context);
assert.ok(rows.every((row) => !row.imported));
disposed();
assert.ok(listenerDisposed);
// 模拟对象未提供写文件、配置更新或执行命令 API；调用它们会直接失败。
console.log("外观状态：缺失、同步、过期、未配置和刷新测试通过");

// 合并只处理本次安装的项目文件；保留其他来源、无效 URI 及原有顺序。
const mixed = ["file:///other/custom.css", "file:///repo/extras/gnome-look.css", "file:///user/.config/adwcode/gnome-look.css", "不是有效的 URI", "file:///other/gnome-look.css", "file:///repo/extras/gnome-menu.js", "file:///repo/extras/gnome-fonts.css"];
const merged = sandbox.mergeCssImports(context, mixed, ["gnome-look.css", "controls-close-only.css"]);
assert.deepEqual(Array.from(merged), ["file:///other/custom.css", "file:///user/.config/adwcode/gnome-look.css", "不是有效的 URI", "file:///other/gnome-look.css", "file:///repo/extras/gnome-fonts.css", "file:///user/.config/adwcode/controls-close-only.css"]);
assert.deepEqual(Array.from(sandbox.mergeCssImports(context, merged, ["gnome-look.css", "controls-close-only.css"])), Array.from(merged));
assert.equal(sandbox.cssImportName({ extensionPath: "/项目" }, "file:///" + encodeURIComponent("项目") + "/extras/gnome-look.css"), "gnome-look.css");
assert.equal(sandbox.cssImportName(context, "https://example.org/gnome-look.css"), undefined);

// 安装命令应调用去重逻辑；拒绝补丁按钮时不执行任何外部命令。
let updates = 0;
imports = ["file:///repo/extras/gnome-look.css", "file:///user/.config/adwcode/gnome-look.css"];
vscode.ConfigurationTarget = { Global: 1 };
vscode.workspace.getConfiguration = () => ({
  get: (key, fallback) => key === "imports" ? imports : fallback,
  async update(key, value, target) { assert.equal(key, "imports"); assert.equal(target, 1); imports = value; updates++; },
});
sandbox.mockFs.promises = {
  async mkdir() {},
  async writeFile(target, data) { files.set(target, data); },
};
vscode.window.showInformationMessage = async () => undefined;
vscode.commands = { async executeCommand() { throw Error("禁止真实命令"); } };
await sandbox.installCss(context, ["gnome-look.css"]);
assert.equal(updates, 1);
assert.deepEqual(Array.from(imports), ["file:///user/.config/adwcode/gnome-look.css"]);
await sandbox.installCss(context, ["gnome-look.css"]);
assert.equal(updates, 1);
console.log("CSS 安装：源码与副本去重、用户加载项保留和重复安装测试通过");

assert.equal(sandbox.pangoFamily("'更纱黑体 UI SC 11'"), "更纱黑体 UI SC");
assert.equal(sandbox.pangoFamily("'Adwaita Sans Bold Italic 10.5'"), "Adwaita Sans");
assert.equal(sandbox.pangoFamily("无效描述"), undefined);
assert.ok(!sandbox.quotedFont('字体"</style>\n').includes('</style>'));
assert.ok(!sandbox.quotedFont('字体"</style>\n').includes('\n'));
// 使用模拟命令验证失败和取消路径，不向真实窗口发送重载命令。
await (async () => {
  assert.equal(sandbox.parseTheme("AdwCode Dark High Contrast"), undefined);
  assert.equal(sandbox.parseTheme("AdwCode 深色 高对比度"), undefined);
  assert.equal(sandbox.parseTheme("AdwCode Dark").kind, "dark");
  assert.equal(sandbox.parseTheme("AdwCode 深色").accent, "blue");
  const variant = sandbox.parseTheme("AdwCode 青色 浅色 · 彩色状态栏");
  assert.equal(variant.kind, "light");
  assert.equal(variant.accent, "teal");
  assert.equal(variant.suffix, " · 彩色状态栏");
  // 第三方的 Adwaita 标签不能被当作本项目主题处理。
  assert.equal(sandbox.parseTheme("Adwaita Dark"), undefined);
  assert.equal(sandbox.labelFor("teal", "light", variant.suffix,
    new Set(["AdwCode 青色 浅色 · 彩色状态栏"])), "AdwCode 青色 浅色 · 彩色状态栏");
  assert.equal(sandbox.labelFor("teal", "light", variant.suffix,
    new Set(["AdwCode 浅色"])), "AdwCode 浅色");
  await sandbox.readSystemFonts();
  const generated = sandbox.cssSource(context, "gnome-fonts.css");
  assert.ok(generated.includes('"更纱黑体 UI SC"'));
  assert.ok(generated.includes('system-ui, sans-serif'));
  sandbox.testContext = context;
  vm.runInContext("extensionContext = testContext", sandbox);
  let reloads = 0;
  let errors = 0;
  let enabled = true;
  let failUpdate = true;
  sandbox.mockFs.promises = {
    async copyFile(source, target) { files.set(target, files.get(source)); },
    async writeFile(target, data) { files.set(target, data); },
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
  // 未完成的加载器更新期间，下一次触发应继续防抖，不并发修改补丁。
  let finishUpdate;
  let updates = 0;
  vscode.commands.executeCommand = async (command) => {
    if (command === "extension.updateCustomCSS") {
      updates++;
      await new Promise((resolve) => { finishUpdate = resolve; });
    }
  };
  callback();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(updates, 1);
  sandbox.scheduleReload(context);
  callback();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(updates, 1);
  finishUpdate();
  await new Promise((resolve) => setImmediate(resolve));
  sandbox.deactivate();
  assert.ok(cleared);
  console.log("自动重载：更新失败、设置关闭和停用清理测试通过（仅使用模拟对象）");
})().catch((error) => { console.error(error); process.exitCode = 1; });

}
main().catch((error) => { console.error(error); process.exitCode = 1; });
