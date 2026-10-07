// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 离线验证状态检测，不加载真实 VS Code，也不修改工作台。
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
function 加载服务(依赖) {
  return require("../扩展/外观服务").创建外观服务({
    vscode: 依赖.require("vscode"), fs: 依赖.require("fs"), os: 依赖.require("os"), path,
    execFile: 依赖.require("child_process").execFile, 环境: 依赖.process.env || {},
  });
}
async function main() {
const files = new Map();
let imports = [];
let output = "";
let shown = 0;
let channelDisposed = false;
const channel = {
  replace(text) { output = text; },
  show() { shown++; },
  dispose() { channelDisposed = true; },
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
  window: { createOutputChannel: () => channel },
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
};
Object.assign(sandbox, 加载服务(sandbox));
const context = { extensionPath: "/repo", subscriptions: [] };
let rows = sandbox.外观安装状态(context);
assert.equal(rows[0].copied, "源文件不可读");
assert.equal(rows[0].patched, "无法读取");
const html = "/app/out/vs/code/electron-browser/workbench/workbench.esm.html";
let patch = "<!-- !! VSCODE-CUSTOM-CSS-START !! -->";
for (const row of rows) {
  files.set(`/repo/附加外观/${row.name}`, `内容：${row.name}`);
  const content = sandbox.样式源码(context, row.name);
  files.set(`/user/.config/adwcode/${row.name}`, content);
  imports.push(`file:///user/.config/adwcode/${row.name}`);
  patch += row.name.endsWith(".js") ? `<script>${content}</script>` : `<style>${content}</style>`;
}
patch += "<!-- !! VSCODE-CUSTOM-CSS-END !! -->";
files.set(html, patch);
rows = sandbox.外观安装状态(context);
assert.ok(rows.every((row) => row.copied === "已同步" && row.imported && row.patched === "磁盘补丁已更新"));
// 未包在加载器标记内的内容不能被误判为已注入。
files.set(html, patch.replace(/<!--.*?-->/g, ""));
assert.ok(sandbox.外观安装状态(context).every((row) => row.patched === "未注入当前版本"));
files.set(html, patch);
// 开发时允许直接加载仓库源码，URI 中的非 ASCII 字符可以编码。
imports = rows.map((row) => `file:///repo/附加外观/${row.name}`);
assert.ok(sandbox.外观安装状态(context).every((row) => row.imported));
imports.push("不是有效的 URI");
assert.ok(sandbox.外观安装状态(context).every((row) => row.imported));
// 源文件和副本同时引用时，不能宣称安装已经就绪。
imports.push("file:///user/.config/adwcode/GNOME外观.css");
assert.equal(sandbox.外观安装状态(context)[0].importCount, 2);
sandbox.显示外观安装状态(context);
assert.ok(output.includes("重复引用（2 项）"));
assert.ok(!output.includes("文件与补丁均已就绪"));
assert.equal(shown, 1);
files.set(html, patch.replace("<!-- !! VSCODE-CUSTOM-CSS-END !! -->", "<style>内容：GNOME外观.css</style><!-- !! VSCODE-CUSTOM-CSS-END !! -->"));
assert.equal(sandbox.外观安装状态(context)[0].patched, "重复注入，补丁待更新");
files.set(html, patch);
imports.pop();
sandbox.显示外观安装状态(context);
assert.ok(output.includes("文件与补丁均已就绪"));
files.set("/repo/附加外观/GNOME外观.css", "新样式");
sandbox.显示外观安装状态(context);
assert.ok(output.includes("副本待更新"));
assert.ok(output.includes("未注入当前版本"));
assert.ok(!output.includes("<"));
assert.equal(shown, 3);
imports = [];
rows = sandbox.外观安装状态(context);
assert.ok(rows.every((row) => !row.imported));
// 输出通道只创建一次并交给 subscriptions 释放；释放后可重新创建。
assert.equal(context.subscriptions.length, 1);
context.subscriptions[0].dispose();
assert.ok(channelDisposed);
sandbox.显示外观安装状态(context);
assert.equal(context.subscriptions.length, 2);
// 模拟对象未提供写文件、配置更新或执行命令 API；调用它们会直接失败。
console.log("外观状态：缺失、同步、过期、未配置和输出刷新测试通过");

// 合并只处理本次安装的项目文件；保留其他来源、无效 URI 及原有顺序。
const mixed = ["file:///other/custom.css", "file:///repo/附加外观/GNOME外观.css", "file:///user/.config/adwcode/GNOME外观.css", "不是有效的 URI", "file:///other/GNOME外观.css", "file:///repo/附加外观/GNOME字体.css"];

// 清理已知目录中的旧引用；同名的其他用户文件必须保留。
mixed.push("file:///repo/extras/gnome-look.css", "file:///repo/附加外观/gnome-look.css", "file:///user/.config/adwcode/window-state.js", "file:///repo/附加外观/gnome-menu.js");
assert.equal(sandbox.识别加载文件(context, "file:///other/gnome-look.css"), undefined);
const unrelated = sandbox.合并加载引用(context, ["file:///other/gnome-look.css"], []);
assert.deepEqual(Array.from(unrelated), ["file:///other/gnome-look.css"]);
const merged = sandbox.合并加载引用(context, mixed, ["GNOME外观.css", "仅关闭窗口控件.css"]);
assert.deepEqual(Array.from(merged), ["file:///other/custom.css", "file:///user/.config/adwcode/GNOME外观.css", "不是有效的 URI", "file:///other/GNOME外观.css", "file:///repo/附加外观/GNOME字体.css", "file:///user/.config/adwcode/仅关闭窗口控件.css"]);
assert.deepEqual(Array.from(sandbox.合并加载引用(context, merged, ["GNOME外观.css", "仅关闭窗口控件.css"])), Array.from(merged));
assert.equal(sandbox.识别加载文件({ extensionPath: "/项目" }, "file:///" + encodeURIComponent("项目") + "/附加外观/GNOME外观.css"), "GNOME外观.css");
assert.equal(sandbox.识别加载文件(context, "https://example.org/GNOME外观.css"), undefined);

// XDG 配置路径须为绝对路径；升级时仍识别旧默认目录引用。
const originalMockFs = sandbox.mockFs;
for (const [xdg, expected] of [["/配置", "/配置/adwcode"], ["relative", "/user/.config/adwcode"], ["", "/user/.config/adwcode"]]) {
  const isolated = { ...sandbox, process: { platform: "linux", env: { XDG_CONFIG_HOME: xdg } }, module: { exports: {} } };
  Object.assign(isolated, 加载服务(isolated));
  assert.deepEqual(Array.from(isolated.合并加载引用(context, ["file:///user/.config/adwcode/GNOME外观.css"], ["GNOME外观.css"])), [`file://${expected}/GNOME外观.css`]);
}
sandbox.mockFs = originalMockFs;

// 安装命令应调用去重逻辑；拒绝补丁按钮时不执行任何外部命令。
let updates = 0;
let workspaceImports;
imports = ["file:///repo/附加外观/GNOME外观.css", "file:///user/.config/adwcode/GNOME外观.css"];
vscode.ConfigurationTarget = { Global: 1 };
vscode.workspace.getConfiguration = () => ({
  get: (key, fallback) => key === "imports" ? workspaceImports || imports : fallback,
  inspect: () => ({ defaultValue: [], globalValue: imports, workspaceValue: workspaceImports }),
  async update(key, value, target) { assert.equal(key, "imports"); assert.equal(target, 1); imports = value; updates++; },
});
sandbox.mockFs.promises = {
  async mkdir() {},
  async writeFile(target, data) { files.set(target, data); },
  async rename(source, target) { files.set(target, files.get(source)); files.delete(source); },
  async unlink(target) { files.delete(target); },
};
vscode.window.showInformationMessage = async () => undefined;
vscode.commands = { async executeCommand() { throw Error("禁止真实命令"); } };
await sandbox.安装样式(context, ["GNOME外观.css"]);
assert.equal(updates, 1);
assert.deepEqual(Array.from(imports), ["file:///user/.config/adwcode/GNOME外观.css"]);
await sandbox.安装样式(context, ["GNOME外观.css"]);
assert.equal(updates, 1);
// 工作区覆盖不能被复制到用户设置；不向错误的有效加载项发送补丁命令。
imports = ["file:///other/user.css"];
workspaceImports = ["file:///other/workspace.css"];
let warnings = [];
vscode.window.showWarningMessage = async message => { warnings.push(message); };
await sandbox.安装样式(context, ["GNOME外观.css"]);
assert.deepEqual(Array.from(imports), ["file:///other/user.css", "file:///user/.config/adwcode/GNOME外观.css"]);
assert.deepEqual(workspaceImports, ["file:///other/workspace.css"]);
assert.ok(warnings.some(message => message.includes("工作区")));
workspaceImports = undefined;
// 未安装加载器时，剪贴板片段必须能直接组成合法 JSON。
let copied;
const getExtension = vscode.extensions.getExtension;
vscode.extensions.getExtension = () => undefined;
vscode.env.clipboard = { async writeText(text) { copied = text; } };
await sandbox.安装样式(context, ["GNOME外观.css"]);
assert.deepEqual(JSON.parse("{" + copied + "}")["vscode_custom_css.imports"], ["file:///other/user.css", "file:///user/.config/adwcode/GNOME外观.css"]);
vscode.extensions.getExtension = getExtension;
console.log("CSS 安装：源码与副本去重、用户加载项保留和重复安装测试通过");

// 配置拒绝时恢复所有副本；选择和移除保留其他加载项。
const 正常配置 = vscode.workspace.getConfiguration;
const oldCopy = files.get("/user/.config/adwcode/GNOME外观.css");
let 拒绝一次 = true;
vscode.workspace.getConfiguration = (...args) => ({ ...正常配置(...args), async update(key, value, target) { if (拒绝一次) { 拒绝一次 = false; throw Error("配置写入被拒绝"); } return 正常配置().update(key, value, target); } });
imports = ["file:///other/user.css"];
await assert.rejects(sandbox.安装样式(context, ["GNOME外观.css"]), /原文件和配置已恢复/);
assert.equal(files.get("/user/.config/adwcode/GNOME外观.css"), oldCopy);
assert.deepEqual(imports, ["file:///other/user.css"]);
assert.ok(![...files.keys()].some(name => name.endsWith(".tmp")));
vscode.workspace.getConfiguration = 正常配置;
const 正常写入 = sandbox.mockFs.promises.writeFile;
let 写入次数 = 0;
sandbox.mockFs.promises.writeFile = async (...args) => { if (++写入次数 === 2) throw Error("磁盘写入失败"); return 正常写入(...args); };
await assert.rejects(sandbox.安装样式(context, ["GNOME外观.css", "GNOME字体.css"]), /磁盘写入失败/);
assert.equal(files.get("/user/.config/adwcode/GNOME外观.css"), oldCopy);
assert.ok(![...files.keys()].some(name => name.endsWith(".tmp")));
sandbox.mockFs.promises.writeFile = 正常写入;

imports = ["file:///other/user.css", "file:///repo/附加外观/GNOME外观.css", "file:///user/.config/adwcode/GNOME字体.css"];
vscode.window.showQuickPick = async () => [{label: "GNOME字体.css"}];
await sandbox.选择外观组件(context);
assert.deepEqual(Array.from(imports), ["file:///other/user.css", "file:///user/.config/adwcode/GNOME字体.css"]);
await sandbox.执行外观事务(context, [], true);
assert.deepEqual(Array.from(imports), ["file:///other/user.css"]);
imports = null;
await assert.rejects(sandbox.选择外观组件(context), /必须是字符串数组/);
assert.throws(() => sandbox.外观安装状态(context), /必须是字符串数组/);
imports = [];
let 顺序 = [];
await Promise.all([sandbox.排队外观操作(async () => { 顺序.push(1); await Promise.resolve(); 顺序.push(2); }), sandbox.排队外观操作(async () => { 顺序.push(3); })]);
assert.deepEqual(顺序, [1,2,3]);
console.log("外观管理：选择、移除、失败回滚及串行操作测试通过");

assert.equal(sandbox.解析Pango字体("'更纱黑体 UI SC 11'"), "更纱黑体 UI SC");
assert.equal(sandbox.解析Pango字体("'Adwaita Sans Bold Italic 10.5'"), "Adwaita Sans");
assert.equal(sandbox.解析Pango字体("无效描述"), undefined);
assert.ok(!sandbox.引用字体名称('字体"</style>\n').includes('</style>'));
assert.ok(!sandbox.引用字体名称('字体"</style>\n').includes('\n'));
await sandbox.读取系统字体();
const generated = sandbox.样式源码(context, "GNOME字体.css");
assert.ok(generated.includes('"更纱黑体 UI SC"'));
assert.ok(generated.includes('system-ui, sans-serif'));
console.log("界面字体：Pango 解析与 CSS 生成测试通过");

}
main().catch((error) => { console.error(error); process.exitCode = 1; });
