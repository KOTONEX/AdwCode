// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//
// JSDoc 类型使用 `import("vscode")` / `import("child_process")` 等写法；
// 检查由 tsconfig.json + 类型声明/ 下的手写最小类型面完成（cargo run --quiet -- 类型检查），
// 扩展本身仍是无构建步骤、无依赖的纯 JavaScript。
// @ts-check
/** @typedef {"unknown" | "not-enabled" | "enabled" | "stale"} CssPatchState */

const vscode = /** @type {typeof import("vscode")} */ (require("vscode"));
const { execFile } = /** @type {typeof import("child_process")} */ (require("child_process"));
const fs = /** @type {typeof import("fs")} */ (require("fs"));
const os = /** @type {typeof import("os")} */ (require("os"));
const path = /** @type {typeof import("path")} */ (require("path"));

const CUSTOM_CSS_EXTENSION = "be5invis.vscode-custom-css";
const 旧配置目录 = path.join(os.homedir(), ".config", "adwcode");
const 配置根目录 = process.env?.XDG_CONFIG_HOME;
const CSS_DIR = 配置根目录 && path.isAbsolute(配置根目录)
  ? path.join(配置根目录, "adwcode") : 旧配置目录;

/**
 * 附加外观/ 中的外观样式与状态脚本，以及各自在补丁 HTML 中的识别标记。
 * @type {Record<string, string>}
 */
const CSS_FILES = {
  "GNOME外观.css": "--vscode-cornerRadius-small",
  "仅关闭窗口控件.css": "window-max-restore",
  "GNOME字体.css": "--adwcode-ui-font",
  "窗口状态.js": "adwcode.windowState",
};

// 仅用于清理旧版引用，不提供旧文件名的功能入口。
const 旧加载文件 = ["gnome-look.css", "controls-close-only.css", "gnome-fonts.css", "window-state.js", "gnome-menu.js"];

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

/** 只识别本扩展源目录及安装目录，保留其他位置的用户文件。
 * @param {import("vscode").ExtensionContext} context
 * @param {string} value @returns {string | undefined}
 */
function 识别加载文件(context, value) {
  try {
    const uri = vscode.Uri.parse(value);
    if (uri.scheme !== "file") return undefined;
    const file = path.resolve(uri.fsPath);
    const name = path.basename(file);
    if (![...Object.keys(CSS_FILES), ...旧加载文件].includes(name)) return undefined;
    // 旧目录仅参与失效引用清理，不作为加载入口。
    const folders = [CSS_DIR, 旧配置目录, path.join(context.extensionPath, "附加外观")];
    if (旧加载文件.includes(name)) folders.push(path.join(context.extensionPath, "extras"));
    return folders.some((folder) =>
      file === path.resolve(folder, name)) ? name : undefined;
  } catch {
    return undefined;
  }
}

/** 将本次安装的组件统一为单份副本引用，保持其他加载项及其顺序。
 * @param {import("vscode").ExtensionContext} context
 * @param {string[]} imports
 * @param {string[]} names @returns {string[]}
 */
function 合并加载引用(context, imports, names) {
  const merged = [];
  const added = new Set();
  for (const value of imports) {
    const name = 识别加载文件(context, value);
    if (name && 旧加载文件.includes(name)) continue;
    if (name && names.includes(name)) {
      if (!added.has(name)) {
        merged.push(vscode.Uri.file(path.join(CSS_DIR, name)).toString());
        added.add(name);
      }
    } else {
      merged.push(value);
    }
  }
  for (const name of names) {
    if (!added.has(name)) {
      merged.push(vscode.Uri.file(path.join(CSS_DIR, name)).toString());
      added.add(name);
    }
  }
  return merged;
}

/** @returns {string | undefined} */
function 工作台HTML路径() {
  const candidates = [
    path.join(vscode.env.appRoot, "out", "vs", "code", "electron-browser", "workbench", "workbench.esm.html"),
    path.join(vscode.env.appRoot, "out", "vs", "code", "electron-browser", "workbench", "workbench.html"),
    path.join(vscode.env.appRoot, "out", "vs", "code", "electron-sandbox", "workbench", "workbench.html"),
  ];
  return candidates.find((candidate) => fs.existsSync(candidate));
}

/**
 * @param {string[]} markers
 * @returns {CssPatchState}
 */
function 样式补丁状态(markers) {
  const html = 工作台HTML路径();
  if (!html) {
    return "unknown";
  }
  let content;
  try {
    content = fs.readFileSync(html, "utf8");
  } catch (error) {
    return "unknown";
  }
  const patch = content.match(/<!-- !! VSCODE-CUSTOM-CSS-START !! -->([\s\S]*?)<!-- !! VSCODE-CUSTOM-CSS-END !! -->/)?.[1];
  if (patch === undefined) {
    return "not-enabled";
  }
  return markers.every((marker) => patch.includes(marker)) ? "enabled" : "stale";
}

/**
 * 只读取安装状态；磁盘补丁与当前窗口的加载状态分别报告。
 * @param {import("vscode").ExtensionContext} context
 * @returns {{name: string, copied: string, imported: boolean, importCount: number, patched: string}[]}
 */
function 外观安装状态(context) {
  const imports = vscode.workspace.getConfiguration("vscode_custom_css").get("imports", /** @type {string[]} */ ([]));
  const htmlPath = 工作台HTML路径();
  let html;
  try {
    html = htmlPath ? fs.readFileSync(htmlPath, "utf8") : undefined;
  } catch {
    html = undefined;
  }
  const patch = html?.match(/<!-- !! VSCODE-CUSTOM-CSS-START !! -->([\s\S]*?)<!-- !! VSCODE-CUSTOM-CSS-END !! -->/)?.[1];
  return Object.keys(CSS_FILES).map((name) => {
    const target = path.join(CSS_DIR, name);
    let source;
    let installed;
    try { source = 样式源码(context, name); } catch { /* 单独报告 */ }
    try { installed = fs.readFileSync(target, "utf8"); } catch { /* 单独报告 */ }
    const injected = source === undefined ? undefined :
      (name.endsWith(".js") ? `<script>${source}</script>` : `<style>${source}</style>`);
    const importCount = imports.filter((value) => 识别加载文件(context, value) === name).length;
    const patchCount = [...(patch || "").matchAll(/<(?:style|script)>([\s\S]*?)<\/(?:style|script)>/g)]
      .filter((match) => match[1].includes(CSS_FILES[name]) || match[0] === injected).length;
    return {
      name,
      copied: source === undefined ? "源文件不可读" : installed === undefined ? "未安装或不可读" :
        installed === source ? "已同步" : "副本待更新",
      imported: importCount > 0,
      importCount,
      patched: html === undefined ? "无法读取" : patchCount > 1 ? "重复注入，补丁待更新" : injected !== undefined && patch?.includes(injected) ?
        "磁盘补丁已更新" : "未注入当前版本",
    };
  });
}

/** @type {import("vscode").OutputChannel | undefined} */
let 外观状态输出通道;

/**
 * 状态输出通道按需创建，登记到 subscriptions 统一释放；释放后允许重新创建。
 * @param {import("vscode").ExtensionContext} context
 * @returns {import("vscode").OutputChannel}
 */
function 外观状态通道(context) {
  if (外观状态输出通道 === undefined) {
    const channel = vscode.window.createOutputChannel("AdwCode 外观状态");
    外观状态输出通道 = channel;
    context.subscriptions.push({ dispose: () => { 外观状态输出通道 = undefined; channel.dispose(); } });
  }
  return 外观状态输出通道;
}

/** 只读取安装状态并写入输出通道；再次执行命令即刷新。
 * @param {import("vscode").ExtensionContext} context @returns {void}
 */
function 显示外观安装状态(context) {
  const rows = 外观安装状态(context);
  const loader = vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION);
  const ready = Boolean(loader) && rows.every((row) => row.copied === "已同步" && row.importCount === 1 && row.patched === "磁盘补丁已更新");
  const labels = /** @type {Record<string, string>} */ ({
    "GNOME外观.css": "工作台外观", "仅关闭窗口控件.css": "窗口按钮", "GNOME字体.css": "界面字体", "窗口状态.js": "窗口状态",
  });
  const lines = [
    "外观状态",
    "========",
    "只检查安装文件与磁盘补丁，不修改配置或重载窗口。再次执行“AdwCode: 查看外观安装状态”即刷新。",
    "",
    `安装准备：${ready ? "文件与补丁均已就绪" : "外观文件需要检查"}`,
    `- 加载器（Custom CSS and JS Loader）：${loader ? "已安装" : "未安装"}`,
    "- 当前窗口：无法直接确认是否已加载；磁盘补丁状态与窗口显示分别检查。",
    "",
    "外观组件：",
  ];
  for (const row of rows) {
    lines.push(
      `[${labels[row.name] || row.name}] ${row.name}`,
      `- 安装副本：${row.copied}`,
      `- 加载器配置：${row.importCount > 1 ? `重复引用（${row.importCount} 项），请重新安装外观` : row.imported ? "已加入" : "未加入"}`,
      `- 磁盘补丁：${row.patched}`,
    );
  }
  let step = 0;
  lines.push("", "下一步：");
  if (!loader) lines.push(`${++step}. 安装 Custom CSS and JS Loader。`);
  lines.push(
    `${++step}. 副本或加载配置需要更新时，执行“AdwCode: 安装 GNOME 外观（CSS）”。`,
    `${++step}. 磁盘补丁需要更新时，首次执行加载器的“Enable Custom CSS and JS”；已启用时执行“Reload Custom CSS and JS”。`,
    `${++step}. 保存工作后手动重载窗口，再检查实际外观。重载可能中断扩展会话或调试任务。`,
    "",
    "此输出只读取状态，不会自动执行这些操作。",
    "",
  );
  const channel = 外观状态通道(context);
  channel.replace(lines.join("\n"));
  channel.show();
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @param {string[]} names
 * @returns {Promise<void>}
 */
async function 安装样式(context, names) {
  return 执行外观事务(context, names, false);
}

/** 串行执行写入命令；失败反馈覆盖配置、剪贴板和加载器调用。
 * @type {Promise<void>}
 */
let 外观操作队列 = Promise.resolve();
/** @param {() => Promise<void>} 操作 @returns {Promise<void>} */
function 排队外观操作(操作) {
  const 本次 = 外观操作队列.then(操作).catch(async (错误) => {
    await vscode.window.showErrorMessage(`AdwCode：外观操作失败：${String(错误)}`);
  });
  外观操作队列 = 本次;
  return 本次;
}

/** @param {import("vscode").ExtensionContext} context
 * @param {string[]} names @param {boolean} 替换全部 @returns {Promise<void>}
 */
async function 执行外观事务(context, names, 替换全部) {
  if (names.some(name => !Object.hasOwn(CSS_FILES, name))) throw Error("未知外观组件");
  await 读取系统字体();
  const loader = vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION);
  const config = vscode.workspace.getConfiguration("vscode_custom_css");
  const inspected = config.inspect("imports");
  const 原全局配置 = inspected?.globalValue;
  const imports = 原全局配置 ?? inspected?.defaultValue ?? [];
  if (!Array.isArray(imports) || !imports.every(value => typeof value === "string")) {
    throw Error("vscode_custom_css.imports 必须是字符串数组");
  }
  const merged = 合并加载引用(context, 替换全部 ? imports.filter(value => !识别加载文件(context, value)) : imports, names);
  /** @type {{target: string, temporary: string, before: string | undefined}[]} */
  const 文件快照 = [];
  let 配置更新开始 = false;
  try {
    if (names.length) await fs.promises.mkdir(CSS_DIR, { recursive: true, mode: 0o700 });
    // 先读取全部源文件和旧副本，再写暂存文件，最后逐项替换。
    const 内容 = names.map(name => ({ name, data: 样式源码(context, name) }));
    for (const {name} of 内容) {
      const target = path.join(CSS_DIR, name);
      文件快照.push({ target, temporary: `${target}.${Date.now()}.${文件快照.length}.tmp`, before: fs.existsSync(target) ? fs.readFileSync(target, "utf8") : undefined });
    }
    for (let index = 0; index < 内容.length; index++) {
      await fs.promises.writeFile(文件快照[index].temporary, 内容[index].data, "utf8");
    }
    for (const item of 文件快照) await fs.promises.rename(item.temporary, item.target);
    if (loader && (merged.length !== imports.length || merged.some((uri, index) => uri !== imports[index]))) {
      配置更新开始 = true;
      await config.update("imports", merged, vscode.ConfigurationTarget.Global);
    }
  } catch (错误) {
    const 恢复错误 = [];
    for (const item of 文件快照) {
      try {
        if (item.before === undefined) {
          if (fs.existsSync(item.target)) await fs.promises.unlink(item.target);
        } else {
          await fs.promises.writeFile(item.temporary, item.before, "utf8");
          await fs.promises.rename(item.temporary, item.target);
        }
      } catch (失败) { 恢复错误.push(String(失败)); }
      try { if (fs.existsSync(item.temporary)) await fs.promises.unlink(item.temporary); }
      catch (失败) { 恢复错误.push(String(失败)); }
    }
    if (配置更新开始) {
      try { await config.update("imports", 原全局配置, vscode.ConfigurationTarget.Global); }
      catch (失败) { 恢复错误.push(String(失败)); }
    }
    throw Error(`${String(错误)}${恢复错误.length ? `；恢复未完成：${恢复错误.join("；")}` : "；原文件和配置已恢复"}`);
  }
  const installed = names.map(name => path.join(CSS_DIR, name));
  const uris = installed.map(file => vscode.Uri.file(file).toString());
  const markers = names.map(name => CSS_FILES[name]);
  const state = 样式补丁状态(markers);
  if (替换全部 && !names.length) {
    if (!loader) await vscode.env.clipboard.writeText('"vscode_custom_css.imports": ' + JSON.stringify(merged, null, 2));
    await vscode.window.showInformationMessage("AdwCode：已移除用户级自有外观引用，保留其他加载项和磁盘副本。工作区覆盖需手动移除；请手动更新加载器并重载窗口。");
    return;
  }
  if (loader) {
    const activeConfig = vscode.workspace.getConfiguration("vscode_custom_css");
    const activeScope = activeConfig.inspect("imports");
    const effective = activeConfig.get("imports", /** @type {string[]} */ ([]));
    const normalized = 合并加载引用(context, effective, names);
    if ((activeScope?.workspaceValue !== undefined || activeScope?.workspaceFolderValue !== undefined) &&
        (normalized.length !== effective.length || normalized.some((value, index) => value !== effective[index]))) {
      await vscode.window.showWarningMessage(
        "AdwCode：外观副本与用户级加载配置已更新，但工作区覆盖了 vscode_custom_css.imports。" +
        "请在工作区中移除该覆盖或手动统一组件引用，再执行加载器命令。工作区配置保持原值。"
      );
      return;
    }
    const [action, command, message] =
      state === "not-enabled"
        ? [
            "启用外观加载器",
            "extension.installCustomCSS",
            `AdwCode：${names.join("、")} 已配置。请执行一次 “Enable Custom CSS and JS” ` +
              `为 VS Code 打补丁（需要 ${vscode.env.appRoot} 的写权限），然后重载窗口。`,
          ]
        : [
            "重新加载外观",
            "extension.updateCustomCSS",
            `AdwCode：外观文件与加载配置已更新。磁盘补丁${state === "enabled" ? "包含这些组件" : state === "unknown" ? "状态无法确认" : "需要更新"}。` +
              `保存工作后，可重新加载以应用 ${names.join("、")}。`,
          ];
    const choice = await vscode.window.showInformationMessage(message, action);
    if (choice === action) {
      await vscode.commands.executeCommand(command);
    }
    return;
  }

  await vscode.env.clipboard.writeText(
    `"vscode_custom_css.imports": ${JSON.stringify(uris, null, 2)}`
  );
  const open = "打开外观目录";
  const choice = await vscode.window.showInformationMessage(
    `AdwCode：已写入 ${installed.join("、")}，并把 vscode_custom_css.imports 片段复制到剪贴板。` +
      `请安装 “Custom CSS and JS Loader”，把片段粘贴到设置中，执行 “Enable Custom CSS and JS” 后重载窗口。`,
    open
  );
  if (choice === open) {
    await vscode.commands.executeCommand("vscode.open", vscode.Uri.file(CSS_DIR));
  }
}

/** @param {import("vscode").ExtensionContext} context @returns {Promise<void>} */
async function 选择外观组件(context) {
  const 当前 = vscode.workspace.getConfiguration("vscode_custom_css").get("imports", /** @type {string[]} */ ([]));
  const 选项 = Object.keys(CSS_FILES).map(name => ({ label: name, picked: 当前.some(value => 识别加载文件(context, value) === name) }));
  const 选择 = await vscode.window.showQuickPick(选项, { canPickMany: true, placeHolder: "选择要加载的外观组件；清空选择可移除全部自有引用" });
  if (选择 !== undefined) await 执行外观事务(context, 选择.map(item => item.label), true);
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("adwcode.查看外观安装状态", async () => { await 读取系统字体(); 显示外观安装状态(context); }),
    vscode.commands.registerCommand("adwcode.安装GNOME外观", () =>
      排队外观操作(() => 安装样式(context, Object.keys(CSS_FILES)))
    ),
    vscode.commands.registerCommand("adwcode.选择外观组件", () => 排队外观操作(() => 选择外观组件(context))),
    vscode.commands.registerCommand("adwcode.移除外观引用", () => 排队外观操作(() => 执行外观事务(context, [], true))),
    vscode.commands.registerCommand("adwcode.安装仅关闭窗口控件", () =>
      排队外观操作(() => 安装样式(context, ["仅关闭窗口控件.css"]))
    ),
  );
}

/** @type {{ activate: typeof activate }} */
module.exports = { activate };
