// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
//
// JSDoc 类型使用 `import("vscode")` / `import("child_process")` 等写法；
// 检查由 tsconfig.json + types/ 下的手写最小类型面完成（meson compile -C builddir 类型检查），
// 扩展本身仍是无构建步骤、无依赖的纯 JavaScript。
// @ts-check
/** @typedef {"blue" | "teal" | "green" | "yellow" | "orange" | "red" | "pink" | "purple" | "slate"} Accent */
/** @typedef {"dark" | "light"} ThemeKind */
/** @typedef {"unknown" | "not-enabled" | "enabled" | "stale"} CssPatchState */

/**
 * 解析后的主题标签。
 * @typedef {object} ParsedTheme
 * @property {string} accent
 * @property {ThemeKind} kind
 * @property {string} suffix
 */

/**
 * globalState 中记录的单项设置。
 * @typedef {object} SettingRecord
 * @property {boolean} wasSet
 * @property {unknown} value
 */

const vscode = /** @type {typeof import("vscode")} */ (require("vscode"));
const { execFile } = /** @type {typeof import("child_process")} */ (require("child_process"));
const fs = /** @type {typeof import("fs")} */ (require("fs"));
const os = /** @type {typeof import("os")} */ (require("os"));
const path = /** @type {typeof import("path")} */ (require("path"));

/** @type {readonly Accent[]} */
const ACCENTS = ["blue", "teal", "green", "yellow", "orange", "red", "pink", "purple", "slate"];
/** @type {Record<Accent, string>} */
const ACCENT_LABELS = {
  blue: "蓝色",
  teal: "青色",
  green: "绿色",
  yellow: "黄色",
  orange: "橙色",
  red: "红色",
  pink: "粉色",
  purple: "紫色",
  slate: "石板灰",
};
/** @type {Record<ThemeKind, string>} */
const MODE_LABELS = { dark: "深色", light: "浅色" };
const PREFIX = "AdwCode ";
// 主题标签仅使用中文，不保留英文标签兼容入口。
/** @type {RegExp[]} */
const THEME_PATTERNS = [
  /^AdwCode (?:(\S+) )?(深色|浅色)(.*)$/,
];
/** @type {Record<string, ThemeKind>} */
const MODE_FROM_LABEL = { 深色: "dark", 浅色: "light" };
/** @type {Record<string, Accent>} */
const ACCENT_FROM_LABEL = {
  蓝色: "blue",
  青色: "teal",
  绿色: "green",
  黄色: "yellow",
  橙色: "orange",
  红色: "red",
  粉色: "pink",
  紫色: "purple",
  石板灰: "slate",
};
const CUSTOM_CSS_EXTENSION = "be5invis.vscode-custom-css";
const CSS_DIR = path.join(os.homedir(), ".config", "adwcode");

/**
 * extras/ 中的外观样式与状态脚本，以及各自在补丁 HTML 中的识别标记。
 * @type {Record<string, string>}
 */
const CSS_FILES = {
  "GNOME外观.css": "--vscode-cornerRadius-small",
  "仅关闭窗口控件.css": "window-max-restore",
  "GNOME字体.css": "--adwcode-ui-font",
  "窗口状态.js": "adwcode.windowState",
};

// 旧文件名只用于移除失效加载引用，不作为可调用的兼容入口。
const RETIRED_CSS_FILES = ["gnome-look.css", "controls-close-only.css", "gnome-fonts.css", "window-state.js", "gnome-menu.js"];

/**
 * “AdwCode: 应用推荐设置” 写入的 GNOME Builder 风格默认值。
 * @type {Record<string, string | boolean | number | null>}
 */
const RECOMMENDED_SETTINGS = {
  // 先写入用户级关闭值，工作区仍可显式覆盖。
  "adwcode.自动重载": false,
  "editor.fontFamily": "Adwaita Mono, monospace",
  "window.autoDetectColorScheme": true,
  "window.autoDetectHighContrast": true,
  "workbench.preferredLightColorTheme": "AdwCode 浅色 · 彩色状态栏",
  "workbench.preferredDarkColorTheme": "AdwCode 深色",
  "workbench.preferredHighContrastLightColorTheme": "AdwCode 浅色 高对比度",
  "workbench.preferredHighContrastColorTheme": "AdwCode 深色 高对比度",
  "workbench.productIconTheme": "adwcode",
  "editor.renderLineHighlight": "none",
  "editor.minimap.enabled": false,
  "editor.guides.indentation": true,
  "editor.stickyScroll.enabled": false,
  "editor.smoothScrolling": true,
  "breadcrumbs.enabled": false,
  "scm.diffDecorations": "none",
  "window.commandCenter": true,
  "window.menuBarVisibility": "compact",
  "window.titleBarStyle": "custom",
  "window.controlsStyle": "native",
  "window.density.editorTabHeight": "compact",
  "workbench.tree.indent": 12,
  "workbench.editor.tabSizing": "shrink",
  "workbench.list.smoothScrolling": true,
  "workbench.iconTheme": null,
};

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
  const source = fs.readFileSync(path.join(context.extensionPath, "extras", name), "utf8");
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
    if (![...Object.keys(CSS_FILES), ...RETIRED_CSS_FILES].includes(name)) return undefined;
    return [CSS_DIR, path.join(context.extensionPath, "extras")].some((folder) =>
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
    if (name && RETIRED_CSS_FILES.includes(name)) continue;
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

/** @type {import("vscode").ExtensionContext | undefined} */
let extensionContext;
/** @type {import("child_process").ChildProcess | undefined} */
let accentMonitor;
let accentSyncSequence = 0;

/**
 * @param {unknown} name
 * @returns {name is string}
 */
function 是否项目主题(name) {
  return typeof name === "string" && name.startsWith(PREFIX) && !/(?:高对比度|High Contrast)/i.test(name);
}

/**
 * @param {unknown} name
 * @returns {ParsedTheme | undefined}
 */
function 解析主题(name) {
  if (!是否项目主题(name)) {
    return undefined;
  }
  for (const pattern of THEME_PATTERNS) {
    const match = name.match(pattern);
    if (match) {
      return {
        accent: ACCENT_FROM_LABEL[match[1]] || (match[1] ? match[1].toLowerCase() : "blue"),
        kind: MODE_FROM_LABEL[match[2]],
        suffix: match[3] || "",
      };
    }
  }
  return undefined;
}

/**
 * @param {Accent} accent
 * @param {ThemeKind} kind
 * @param {string} suffix
 * @param {Set<string>} available
 * @returns {string | undefined}
 */
function 查找主题标签(accent, kind, suffix, available) {
  const kindLabel = MODE_LABELS[kind];
  const accentPart = accent === "blue" ? "" : `${ACCENT_LABELS[accent]} `;
  const candidates = [
    `AdwCode ${accentPart}${kindLabel}${suffix}`,
    `AdwCode ${kindLabel}${suffix}`,
    `AdwCode ${accentPart}${kindLabel}`,
    `AdwCode ${kindLabel}`,
  ];
  return candidates.find((label) => available.has(label));
}

/** @returns {Set<string>} */
function 已安装主题() {
  const extension = vscode.extensions.getExtension("KOTONEX.AdwCode");
  /** @type {Array<{ label: string }>} */
  const themes =
    extension && extension.packageJSON && extension.packageJSON.contributes
      ? extension.packageJSON.contributes.themes || []
      : [];
  return new Set(themes.map((theme) => theme.label));
}

/** @returns {Promise<Accent | undefined>} */
function 读取系统强调色() {
  return new Promise((resolve) => {
    execFile(
      "gsettings",
      ["get", "org.gnome.desktop.interface", "accent-color"],
      { timeout: 5000 },
      (error, stdout) => {
        if (error) {
          resolve(undefined);
          return;
        }
        const value = String(stdout).trim().replace(/^'|'$/g, "");
        resolve(ACCENTS.find((accent) => accent === value));
      }
    );
  });
}

/**
 * @param {boolean} [announce]
 * @returns {Promise<void>}
 */
async function 同步强调色(announce = false) {
  const context = extensionContext;
  const sequence = ++accentSyncSequence;
  if (!context) return;
  const config = vscode.workspace.getConfiguration("adwcode");
  const autoAccent = /** @type {boolean} */ (config.get("自动强调色", true));
  if (autoAccent === false) {
    停止强调色监听();
    if (announce) {
      vscode.window.showInformationMessage("AdwCode：自动强调色已禁用。");
    }
    return;
  }

  const workbench = vscode.workspace.getConfiguration("workbench");
  const current = workbench.get("colorTheme");
  const preferredDark = workbench.get("preferredDarkColorTheme");
  const preferredLight = workbench.get("preferredLightColorTheme");
  if (![current, preferredDark, preferredLight].some(是否项目主题)) {
    停止强调色监听();
    return;
  }
  启动强调色监听();

  const accent = await 读取系统强调色();
  if (extensionContext !== context || sequence !== accentSyncSequence) return;
  if (!vscode.workspace.getConfiguration("adwcode").get("自动强调色", true)) {
    停止强调色监听();
    return;
  }
  if (!accent) {
    if (announce) {
      vscode.window.showWarningMessage("AdwCode：无法获取 GNOME 强调色。");
    }
    return;
  }

  const currentWorkbench = vscode.workspace.getConfiguration("workbench");
  const available = 已安装主题();
  let supported = false;
  let candidateFound = false;
  let workspaceOverride = false;
  /** @type {Array<Thenable<void>>} */
  const updates = [];
  /** @type {Array<[unknown, ThemeKind | undefined, string]>} */
  const candidates = [
    [current, undefined, "colorTheme"],
    [preferredDark, "dark", "preferredDarkColorTheme"],
    [preferredLight, "light", "preferredLightColorTheme"],
  ];
  for (const [name, kind, setter] of candidates) {
    // 读取系统设置期间，操作者可能已经切换主题；不要覆盖更新后的值。
    if (currentWorkbench.get(setter) !== name) continue;
    const parsed = 解析主题(name);
    if (!parsed) {
      continue;
    }
    candidateFound = true;
    const label = 查找主题标签(accent, kind || parsed.kind, parsed.suffix, available);
    supported ||= 解析主题(label)?.accent === accent;
    if (label && label !== name) {
      const inspected = currentWorkbench.inspect(setter);
      if (inspected?.workspaceValue !== undefined || inspected?.workspaceFolderValue !== undefined) {
        workspaceOverride = true;
        continue;
      }
      updates.push(currentWorkbench.update(setter, label, vscode.ConfigurationTarget.Global));
    }
  }

  if (updates.length > 0) await Promise.all(updates);
  if (announce && workspaceOverride) {
    vscode.window.showWarningMessage("AdwCode：已保留工作区覆盖的主题配置；请手动调整工作区中的主题值。" +
      (updates.length ? "其他用户级主题已同步。" : "用户级原值未被覆盖。"));
  } else if (announce && updates.length > 0) {
    vscode.window.showInformationMessage(`AdwCode：已切换到${ACCENT_LABELS[accent]}强调色。`);
  } else if (announce && candidateFound && !workspaceOverride) {
    vscode.window.showInformationMessage(
      supported ? "AdwCode：主题已与系统强调色一致。" : `AdwCode：未安装${ACCENT_LABELS[accent]}强调色变体。`
    );
  }
}

/** @returns {void} */
function 启动强调色监听() {
  if (accentMonitor || process.platform !== "linux") {
    return;
  }
  const child = execFile("gsettings", ["monitor", "org.gnome.desktop.interface", "accent-color"]);
  const stdout = /** @type {import("stream").Readable} */ (child.stdout);
  stdout.on("data", () => {
    同步强调色().catch(() => undefined);
  });
  child.on("error", () => {
    if (accentMonitor === child) {
      accentMonitor = undefined;
    }
  });
  child.on("exit", () => {
    if (accentMonitor !== child) {
      return;
    }
    accentMonitor = undefined;
    setTimeout(() => {
      if (extensionContext) {
        同步强调色().catch(() => undefined);
      }
    }, 5000);
  });
  accentMonitor = child;
}

/** @returns {void} */
function 停止强调色监听() {
  if (accentMonitor) {
    accentMonitor.kill();
    accentMonitor = undefined;
  }
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

/** @param {string} value @returns {string} */
function 转义HTML(value) {
  return value.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char] || char));
}

/** @param {import("vscode").ExtensionContext} context @returns {void} */
function 显示外观安装状态(context) {
  const panel = vscode.window.createWebviewPanel("adwcode.查看外观安装状态", "AdwCode 外观状态", vscode.ViewColumn.One, { enableScripts: true });
  let disposed = false;
  const render = (refreshed = false) => {
    const rows = 外观安装状态(context);
    const loader = vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION);
    const ready = Boolean(loader) && rows.every((row) => row.copied === "已同步" && row.importCount === 1 && row.patched === "磁盘补丁已更新");
    const nonce = Math.random().toString(36).slice(2);
    const labels = /** @type {Record<string, string>} */ ({
      "GNOME外观.css": "工作台外观", "仅关闭窗口控件.css": "窗口按钮", "GNOME字体.css": "界面字体", "窗口状态.js": "窗口状态",
    });
    panel.webview.html = `<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1">
      <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
      <style>
        * { box-sizing: border-box; }
        body { max-width: 800px; margin: 0 auto; padding: 24px 16px; color: var(--vscode-foreground); background: var(--vscode-panel-background); font-family: ${界面字体栈()}; line-height: 1.6; }
        header { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; justify-content: space-between; }
        h1 { font-size: 24px; line-height: 1.3; margin: 0; } h2 { font-size: 16px; margin: 24px 0 8px; }
        h3 { font-size: 15px; margin: 0 0 4px; } p { margin: 8px 0; }
        .muted, dt { color: var(--vscode-descriptionForeground); }
        .card { background: var(--vscode-editorWidget-background); border: 1px solid var(--vscode-editorGroup-border, transparent); border-radius: 15px; padding: 16px; margin: 8px 0; }
        .summary { border-inline-start: 3px solid var(--vscode-focusBorder); }
        .files { border-radius: 15px; background: var(--vscode-editorWidget-background); border: 1px solid var(--vscode-editorGroup-border, transparent); }
        article { padding: 16px; } article + article { border-top: 1px solid var(--vscode-editorGroup-border); }
        code { overflow-wrap: anywhere; } dl { margin: 8px 0 0; } .row { display: grid; grid-template-columns: minmax(96px, 1fr) minmax(0, 2fr); gap: 8px; padding: 4px 0; }
        dd { margin: 0; overflow-wrap: anywhere; } ol { padding-inline-start: 24px; } li + li { margin-top: 8px; }
        button { font: inherit; background: var(--vscode-button-secondaryBackground); color: var(--vscode-button-secondaryForeground); border: 1px solid var(--vscode-button-secondaryBorder, transparent); border-radius: 9px; padding: 6px 14px; cursor: pointer; }
        button:hover { background: var(--vscode-button-secondaryHoverBackground); }
        button:active { background: var(--vscode-toolbar-activeBackground); }
        button:focus-visible { outline: 2px solid var(--vscode-focusBorder); outline-offset: 2px; }
        .vscode-high-contrast .card, .vscode-high-contrast .files, .vscode-high-contrast-light .card, .vscode-high-contrast-light .files { border-color: var(--vscode-contrastBorder); }
        @media (max-width: 360px) { .row { grid-template-columns: 1fr; gap: 0; } }
      </style></head><body>
      <header><h1>外观状态</h1><button id="refresh">刷新状态</button></header>
      <p class="muted">只检查安装文件与磁盘补丁，不修改配置或重载窗口。</p>
      <p role="status" aria-live="polite">${refreshed ? "状态已刷新。" : ""}</p>
      <section aria-labelledby="summary"><h2 id="summary">安装准备</h2><div class="card summary">
      <h3>${ready ? "文件与补丁均已就绪" : "外观文件需要检查"}</h3>
      <dl><div class="row"><dt>加载器</dt><dd>Custom CSS and JS Loader：${loader ? "已安装" : "未安装"}</dd></div>
      <div class="row"><dt>当前窗口</dt><dd>无法直接确认是否已加载。磁盘补丁状态与窗口显示分别检查。</dd></div></dl></div></section>
      <section aria-labelledby="files"><h2 id="files">外观组件</h2><div class="files">
      ${rows.map((row) => `<article><h3>${labels[row.name] || 转义HTML(row.name)}</h3><code class="muted">${转义HTML(row.name)}</code><dl>
      <div class="row"><dt>安装副本</dt><dd>${row.copied}</dd></div>
      <div class="row"><dt>加载器配置</dt><dd>${row.importCount > 1 ? `重复引用（${row.importCount} 项），请重新安装外观` : row.imported ? "已加入" : "未加入"}</dd></div>
      <div class="row"><dt>磁盘补丁</dt><dd>${row.patched}</dd></div></dl></article>`).join("")}
      </div></section>
      <section aria-labelledby="next"><h2 id="next">下一步</h2><div class="card"><ol>
      ${!loader ? '<li>安装 Custom CSS and JS Loader。</li>' : ""}
      <li>副本或加载配置需要更新时，执行“AdwCode: 安装 GNOME 外观（CSS）”。</li>
      <li>磁盘补丁需要更新时，首次执行加载器的“Enable Custom CSS and JS”；已启用时执行“Reload Custom CSS and JS”。</li>
      <li>保存工作后手动重载窗口，再检查实际外观。重载可能中断扩展会话或调试任务。</li>
      </ol><p class="muted">此面板不会自动执行这些操作。</p></div></section>
      <script nonce="${nonce}">const api = acquireVsCodeApi(); const button = document.getElementById('refresh'); button.addEventListener('click', () => api.postMessage('refresh')); ${refreshed ? "button.focus();" : ""}</script>
      </body></html>`;
  };
  const listener = panel.webview.onDidReceiveMessage(async (message) => {
    if (message === "refresh") {
      await 读取系统字体();
      if (!disposed) render(true);
    }
  });
  panel.onDidDispose(() => { disposed = true; listener.dispose(); });
  render();
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @param {string[]} names
 * @returns {Promise<void>}
 */
async function 安装样式(context, names) {
  /** @type {string[]} */
  const installed = [];
  await 读取系统字体();
  try {
    await fs.promises.mkdir(CSS_DIR, { recursive: true });
    for (const name of names) {
      const target = path.join(CSS_DIR, name);
      await fs.promises.writeFile(target, 样式源码(context, name), "utf8");
      installed.push(target);
    }
  } catch (error) {
    const message = /** @type {Error} */ (error).message;
    vscode.window.showErrorMessage(`AdwCode：无法写入外观文件：${message}`);
    return;
  }

  const uris = installed.map((file) => vscode.Uri.file(file).toString());
  const markers = names.map((name) => CSS_FILES[name]);
  const loader = vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION);
  const state = 样式补丁状态(markers);

  if (loader) {
    const config = vscode.workspace.getConfiguration("vscode_custom_css");
    const inspected = config.inspect("imports");
    const imports = /** @type {string[]} */ (inspected?.globalValue ?? inspected?.defaultValue ?? []);
    const merged = 合并加载引用(context, imports, names);
    if (merged.length !== imports.length || merged.some((uri, index) => uri !== imports[index])) {
      await config.update("imports", merged, vscode.ConfigurationTarget.Global);
    }
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

let settingsRunning = false;

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {Promise<void>}
 */
async function 应用推荐设置(context) {
  if (settingsRunning) return;
  settingsRunning = true;
  try {
    await 读取系统字体();
    const candidates = Object.entries({ ...RECOMMENDED_SETTINGS,
      "editor.fontFamily": systemFonts.mono ? `${引用字体名称(systemFonts.mono)}, "Adwaita Mono", monospace` : RECOMMENDED_SETTINGS["editor.fontFamily"],
    });
    /** @type {string[]} */
    const unavailable = [];
    const entries = candidates.filter(([key]) => {
      const [section, name] = 拆分设置键(key);
      const inspected = vscode.workspace.getConfiguration(section).inspect(name);
      // 旧配置中可能留有未注册的用户键，只把有默认定义的推荐项视为可用。
      if (!inspected || inspected.defaultValue === undefined) {
        unavailable.push(key);
        return false;
      }
      return true;
    });
    const preview = entries.map(([key, value]) => `  ${key}: ${JSON.stringify(value)}`).join("\n");
    const apply = "应用";
    const choice = await vscode.window.showInformationMessage(
      `AdwCode 将把 ${entries.length} 项用户设置改为 GNOME Builder 风格，用户级自动重载将关闭。\n` +
        `工作区设置可能覆盖这些用户值。\n\n${preview}` +
        (unavailable.length ? `\n\n当前 VS Code 未提供以下设置，已跳过：${unavailable.join("、")}` : ""),
      { modal: true },
      apply
    );
    if (choice !== apply) {
      return;
    }

    // globalState 以 JSON 序列化，undefined 会被丢弃，因此显式记录"原先是否设置过"
    /** @type {Record<string, SettingRecord>} */
    const previous = /** @type {Record<string, SettingRecord>} */ (context.globalState.get("adwcode.previousSettings") || {});
    for (const [key, value] of entries) {
      const [section, name] = 拆分设置键(key);
      const config = vscode.workspace.getConfiguration(section);
      const inspected = config.inspect(name);
      const oldValue = inspected ? inspected.globalValue : undefined;
      if (!Object.prototype.hasOwnProperty.call(previous, key)) {
        previous[key] = { wasSet: oldValue !== undefined, value: oldValue === undefined ? null : oldValue };
      }
      // 每次写配置前持久化原值，部分失败或重复应用不能丢失首次备份。
      await context.globalState.update("adwcode.previousSettings", { ...previous });
      await config.update(name, value, vscode.ConfigurationTarget.Global);
    }

    const reloadEnabled = vscode.workspace.getConfiguration("adwcode").get("自动重载", false);
    await vscode.window.showInformationMessage(
      "AdwCode：推荐设置已应用。" +
        (reloadEnabled ? "用户级自动重载已关闭，但工作区仍开启了该项，请在工作区设置中关闭。" : "自动重载已关闭。") +
        "标题栏和窗口控件等配置可能需要重载；" +
        "请保存工作并结束扩展会话后手动重载窗口。"
    );
  } catch (error) {
    vscode.window.showErrorMessage(`AdwCode：推荐设置未全部应用，已保留原值供恢复：${/** @type {Error} */ (error).message}`);
  } finally {
    settingsRunning = false;
  }
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {Promise<void>}
 */
async function 恢复推荐设置(context) {
  if (settingsRunning) return;
  settingsRunning = true;
  try {
    /** @type {Record<string, SettingRecord> | undefined} */
    const previous = context.globalState.get("adwcode.previousSettings");
    if (!previous) {
      vscode.window.showInformationMessage("AdwCode：没有可恢复的设置。");
      return;
    }
    for (const [key, record] of Object.entries(previous)) {
      const [section, name] = 拆分设置键(key);
      const target = record && record.wasSet ? record.value : undefined;
      await vscode.workspace.getConfiguration(section).update(
        name,
        target,
        vscode.ConfigurationTarget.Global
      );
      delete previous[key];
      await context.globalState.update("adwcode.previousSettings", { ...previous });
    }
    await context.globalState.update("adwcode.previousSettings", undefined);
    vscode.window.showInformationMessage("AdwCode：已恢复原有设置。");
  } catch (error) {
    vscode.window.showErrorMessage(`AdwCode：设置未全部恢复，剩余记录已保留，可再次执行恢复：${/** @type {Error} */ (error).message}`);
  } finally {
    settingsRunning = false;
  }
}

/**
 * @param {string} key
 * @returns {[string, string]}
 */
function 拆分设置键(key) {
  const index = key.indexOf(".");
  return [key.slice(0, index), key.slice(index + 1)];
}

/** @type {import("vscode").FileSystemWatcher[]} */
let reloadWatchers = [];
/** @type {ReturnType<typeof setTimeout> | undefined} */
let reloadTimer;
let reloadRunning = false;
let reloadGeneration = 0;

/**
 * 重新应用 Custom CSS 并重载窗口（开发时让样式/主题/代码改动立即生效）。
 * @param {import("vscode").ExtensionContext} context
 * @returns {Promise<void>}
 */
async function 应用样式并重载(context) {
  const generation = reloadGeneration;
  const allowed = () => extensionContext === context && generation === reloadGeneration && vscode.workspace.getConfiguration("adwcode").get("自动重载", false);
  if (!allowed()) return;
  await 读取系统字体();
  if (!allowed()) return;
  try {
    // 加载器读取安装目录中的副本，只同步用户已安装的样式。
    for (const name of Object.keys(CSS_FILES)) {
      if (!allowed()) return;
      const target = path.join(CSS_DIR, name);
      if (fs.existsSync(target)) {
        await fs.promises.writeFile(target, 样式源码(context, name), "utf8");
      }
    }
  } catch (error) {
    const message = /** @type {Error} */ (error).message;
    vscode.window.showErrorMessage(`AdwCode：无法同步 CSS 文件：${message}`);
    return;
  }
  // 文件写入会让出执行权；停用或关闭设置后不能继续修改工作台补丁。
  if (!allowed()) return;
  try {
    // Custom CSS and JS Loader 会把 imports 中的样式重新内联进 workbench.html
    await vscode.commands.executeCommand("extension.updateCustomCSS");
  } catch {
    // 已安装加载器但更新失败时，不把错误当成“未安装”，避免无效重载。
    if (vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION)) {
      vscode.window.showErrorMessage("AdwCode：Custom CSS 更新失败，已取消自动重载。请手动检查加载器。");
      return;
    }
  }
  if (!allowed()) {
    return;
  }
  await vscode.commands.executeCommand("workbench.action.reloadWindow");
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function 安排重载(context) {
  if (reloadTimer !== undefined) {
    clearTimeout(reloadTimer);
  }
  reloadTimer = setTimeout(() => {
    reloadTimer = undefined;
    // 加载器会恢复备份并重新写入 HTML，不允许两次更新同时执行。
    if (reloadRunning) {
      安排重载(context);
      return;
    }
    reloadRunning = true;
    应用样式并重载(context).catch(() => undefined).finally(() => {
      reloadRunning = false;
    });
  }, 1500);
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function 启动文件监视(context) {
  if (reloadWatchers.length > 0) {
    return;
  }
  for (const pattern of ["extension/扩展.js", "extras/*.css", "extras/*.js", "themes/*.json", "package.json"]) {
    const watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(context.extensionPath, pattern)
    );
    watcher.onDidChange(() => 安排重载(context));
    watcher.onDidCreate(() => 安排重载(context));
    watcher.onDidDelete(() => 安排重载(context));
    reloadWatchers.push(watcher);
  }
}

/** @returns {void} */
function 停止文件监视() {
  // 即使随后重新开启，也不能恢复上一次已经取消的异步更新。
  reloadGeneration++;
  for (const watcher of reloadWatchers) {
    watcher.dispose();
  }
  reloadWatchers = [];
  if (reloadTimer !== undefined) {
    clearTimeout(reloadTimer);
    reloadTimer = undefined;
  }
}

/**
 * 按 `adwcode.自动重载` 设置启停文件监视。
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function 同步文件监视(context) {
  const enabled = /** @type {boolean} */ (
    vscode.workspace.getConfiguration("adwcode").get("自动重载", false)
  );
  if (enabled) {
    启动文件监视(context);
  } else {
    停止文件监视();
  }
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function activate(context) {
  if (process.platform !== "linux") {
    return;
  }
  extensionContext = context;

  context.subscriptions.push(
    vscode.commands.registerCommand("adwcode.查看外观安装状态", async () => { await 读取系统字体(); 显示外观安装状态(context); }),
    vscode.commands.registerCommand("adwcode.同步强调色", () => 同步强调色(true)),
    vscode.commands.registerCommand("adwcode.安装GNOME外观", () =>
      安装样式(context, Object.keys(CSS_FILES))
    ),
    vscode.commands.registerCommand("adwcode.安装仅关闭窗口控件", () =>
      安装样式(context, ["仅关闭窗口控件.css"])
    ),
    vscode.commands.registerCommand("adwcode.应用推荐设置", () =>
      应用推荐设置(context)
    ),
    vscode.commands.registerCommand("adwcode.恢复推荐设置", () =>
      恢复推荐设置(context)
    ),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("adwcode.自动重载")) {
        同步文件监视(context);
      }
      if (
        event.affectsConfiguration("workbench.colorTheme") ||
        event.affectsConfiguration("workbench.preferredDarkColorTheme") ||
        event.affectsConfiguration("workbench.preferredLightColorTheme") ||
        event.affectsConfiguration("adwcode.自动强调色")
      ) {
        同步强调色().catch(() => undefined);
      }
    }),
    { dispose: 停止强调色监听 },
    { dispose: 停止文件监视 }
  );

  同步强调色().catch(() => undefined);
  同步文件监视(context);
}

/** @returns {void} */
function deactivate() {
  extensionContext = undefined;
  停止强调色监听();
  停止文件监视();
}

/** @type {{ activate: typeof activate, deactivate: typeof deactivate }} */
module.exports = { activate, deactivate };
