// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// @ts-check
/**
 * @param {typeof import("vscode")} vscode
 * @param {typeof import("fs")} fs
 * @param {typeof import("path")} path
 * @param {typeof import("child_process").execFile} execFile
 */
function 创建字体(vscode, execFile, fs, path) {
/** @type {{ ui?: string, mono?: string }} */
let systemFonts = {};

/** 从 Pango 字体描述中取出字体族，不把字号或样式写进 CSS。
 * @param {string} description @returns {string | undefined}
 */
function 解析Pango字体(description) {
  const value = description.trim().replace(/^'|'$/g, "").replace(/\\(['\\])/g, "$1");
  if (!/\s+\d+(?:\.\d+)?$/.test(value)) return undefined;
  return value.replace(/\s+\d+(?:\.\d+)?$/, "")
    .replace(/(?:\s+(?:Bold|Semi-Bold|Semibold|Italic|Oblique|Regular|Medium|Light))+$/i, "").trim() || undefined;
}

/** @param {string} value @returns {string} */
function 引用字体名称(value) {
  return '"' + value.replace(/[\\"\x00-\x1f<>]/g, (char) => `\\${char.charCodeAt(0).toString(16)} `) + '"';
}

/** @returns {Promise<void>} */
async function 读取系统字体() {
  const read = (/** @type {string} */ key) => new Promise((resolve) => {
    execFile("gsettings", ["get", "org.gnome.desktop.interface", key], { timeout: 5000 },
      (error, stdout) => resolve(error ? undefined : 解析Pango字体(String(stdout))));
  });
  const [ui, mono] = await Promise.all([read("font-name"), read("monospace-font-name")]);
  systemFonts = { ui: /** @type {string | undefined} */ (ui), mono: /** @type {string | undefined} */ (mono) };
}

/** @returns {string} */
function 界面字体栈() {
  const configured = vscode.workspace.getConfiguration("adwcode").get("界面字体", "");
  const family = typeof configured === "string" && configured.trim() ? configured.trim() : systemFonts.ui;
  return (family ? `${引用字体名称(family)}, ` : "") + '"Adwaita Sans", "Cantarell", system-ui, sans-serif';
}

/** 生成字体适配文件；其他外观文件原样读取，状态检查也使用同一份预期内容。
 * @param {import("vscode").ExtensionContext} context
 * @param {string} name @returns {string}
 */
function 样式源码(context, name) {
  const source = fs.readFileSync(path.join(context.extensionPath, "附加外观", name), "utf8");
  if (name !== "GNOME字体.css") return source;
  return source + `\n:root, .monaco-workbench { --adwcode-ui-font: ${界面字体栈()}; }\n`;
}


return { 解析Pango字体, 引用字体名称, 读取系统字体, 样式源码 };
}
module.exports = { 创建字体 };
