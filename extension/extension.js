// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
//
// JSDoc 类型使用 `import("vscode")` / `import("child_process")` 等写法；
// 检查由 tsconfig.json + types/ 下的手写最小类型面完成（make typecheck），
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
const PREFIX = "Adwaita ";
const HIGH_CONTRAST = "高对比度";
// 主题标签使用中文，同时兼容旧版英文标签。
/** @type {RegExp[]} */
const THEME_PATTERNS = [
  /^Adwaita (?:(\S+) )?(深色|浅色)(.*)$/,
  /^Adwaita (?:(\w+) )?(Dark|Light)(.*)$/,
];
/** @type {Record<string, ThemeKind>} */
const MODE_FROM_LABEL = { 深色: "dark", 浅色: "light", Dark: "dark", Light: "light" };
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
 * extras/ 中的 CSS 文件，以及各自在补丁 HTML 中的识别标记。
 * @type {Record<string, string>}
 */
const CSS_FILES = {
  "gnome-look.css": "--vscode-cornerRadius-small",
  "controls-close-only.css": "window-max-restore",
};

/**
 * “Adwaita: Apply Recommended Settings” 写入的 GNOME/Builder 风格默认值。
 * @type {Record<string, string | boolean | number | null>}
 */
const RECOMMENDED_SETTINGS = {
  "adwcode.autoReload": true,
  "editor.fontFamily": "Adwaita Mono, monospace",
  "editor.renderLineHighlight": "none",
  "editor.minimap.enabled": false,
  "editor.guides.indentation": true,
  "editor.stickyScroll.enabled": false,
  "editor.smoothScrolling": true,
  "breadcrumbs.enabled": false,
  "scm.diffDecorations": "none",
  "window.commandCenter": false,
  "window.menuBarVisibility": "compact",
  "window.density.editorTabHeight": "compact",
  "workbench.tree.indent": 12,
  "workbench.editor.tabSizing": "shrink",
  "workbench.list.smoothScrolling": true,
  "workbench.iconTheme": null,
};

/** @type {import("vscode").ExtensionContext | undefined} */
let extensionContext;
/** @type {import("child_process").ChildProcess | undefined} */
let accentMonitor;

/**
 * @param {unknown} name
 * @returns {name is string}
 */
function isOurTheme(name) {
  return typeof name === "string" && name.startsWith(PREFIX) && !name.includes(HIGH_CONTRAST);
}

/**
 * @param {unknown} name
 * @returns {ParsedTheme | undefined}
 */
function parseTheme(name) {
  if (!isOurTheme(name)) {
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
function labelFor(accent, kind, suffix, available) {
  const kindLabel = MODE_LABELS[kind];
  const accentPart = accent === "blue" ? "" : `${ACCENT_LABELS[accent]} `;
  const candidates = [
    `Adwaita ${accentPart}${kindLabel}${suffix}`,
    `Adwaita ${kindLabel}${suffix}`,
    `Adwaita ${accentPart}${kindLabel}`,
    `Adwaita ${kindLabel}`,
  ];
  return candidates.find((label) => available.has(label));
}

/** @returns {Set<string>} */
function availableThemes() {
  const extension = vscode.extensions.getExtension("KOTONEX.AdwCode");
  /** @type {Array<{ label: string }>} */
  const themes =
    extension && extension.packageJSON && extension.packageJSON.contributes
      ? extension.packageJSON.contributes.themes || []
      : [];
  return new Set(themes.map((theme) => theme.label));
}

/** @returns {Promise<Accent | undefined>} */
function readSystemAccent() {
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
async function syncAccent(announce = false) {
  const config = vscode.workspace.getConfiguration("adwcode");
  const autoAccent = /** @type {boolean} */ (config.get("autoAccent", true));
  if (autoAccent === false) {
    stopAccentMonitor();
    if (announce) {
      vscode.window.showInformationMessage("Adwaita：自动强调色已禁用。");
    }
    return;
  }

  const workbench = vscode.workspace.getConfiguration("workbench");
  const current = workbench.get("colorTheme");
  const preferredDark = workbench.get("preferredDarkColorTheme");
  const preferredLight = workbench.get("preferredLightColorTheme");
  if (![current, preferredDark, preferredLight].some(isOurTheme)) {
    stopAccentMonitor();
    return;
  }
  startAccentMonitor();

  const accent = await readSystemAccent();
  if (!accent) {
    if (announce) {
      vscode.window.showWarningMessage("Adwaita：无法获取 GNOME 强调色。");
    }
    return;
  }

  const available = availableThemes();
  /** @type {Array<Thenable<void>>} */
  const updates = [];
  /** @type {Array<[unknown, ThemeKind | undefined, string]>} */
  const candidates = [
    [current, undefined, "colorTheme"],
    [preferredDark, "dark", "preferredDarkColorTheme"],
    [preferredLight, "light", "preferredLightColorTheme"],
  ];
  for (const [name, kind, setter] of candidates) {
    const parsed = parseTheme(name);
    if (!parsed) {
      continue;
    }
    const label = labelFor(accent, kind || parsed.kind, parsed.suffix, available);
    if (label && label !== name) {
      updates.push(workbench.update(setter, label, vscode.ConfigurationTarget.Global));
    }
  }

  if (updates.length > 0) {
    await Promise.all(updates);
    if (announce) {
      vscode.window.showInformationMessage(`Adwaita：已切换到${ACCENT_LABELS[accent]}强调色。`);
    }
  } else if (announce) {
    vscode.window.showInformationMessage(
      `Adwaita：未安装${ACCENT_LABELS[accent]}强调色变体。`
    );
  }
}

/** @returns {void} */
function startAccentMonitor() {
  if (accentMonitor || process.platform !== "linux") {
    return;
  }
  const child = execFile("gsettings", ["monitor", "org.gnome.desktop.interface", "accent-color"]);
  const stdout = /** @type {import("stream").Readable} */ (child.stdout);
  stdout.on("data", () => {
    syncAccent().catch(() => undefined);
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
        syncAccent().catch(() => undefined);
      }
    }, 5000);
  });
  accentMonitor = child;
}

/** @returns {void} */
function stopAccentMonitor() {
  if (accentMonitor) {
    accentMonitor.kill();
    accentMonitor = undefined;
  }
}

/** @returns {string | undefined} */
function workbenchHtmlPath() {
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
function cssPatchState(markers) {
  const html = workbenchHtmlPath();
  if (!html) {
    return "unknown";
  }
  let content;
  try {
    content = fs.readFileSync(html, "utf8");
  } catch (error) {
    return "unknown";
  }
  if (!content.includes("VSCODE-CUSTOM-CSS-START")) {
    return "not-enabled";
  }
  return markers.every((marker) => content.includes(marker)) ? "enabled" : "stale";
}

/**
 * 只读取安装状态；磁盘补丁与当前窗口的加载状态分别报告。
 * @param {import("vscode").ExtensionContext} context
 * @returns {{name: string, copied: string, imported: boolean, patched: string}[]}
 */
function appearanceStatus(context) {
  const imports = vscode.workspace.getConfiguration("vscode_custom_css").get("imports", /** @type {string[]} */ ([]));
  const htmlPath = workbenchHtmlPath();
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
    try { source = fs.readFileSync(path.join(context.extensionPath, "extras", name), "utf8"); } catch { /* 单独报告 */ }
    try { installed = fs.readFileSync(target, "utf8"); } catch { /* 单独报告 */ }
    const injected = source === undefined ? undefined :
      (name.endsWith(".js") ? `<script>${source}</script>` : `<style>${source}</style>`);
    return {
      name,
      copied: source === undefined ? "源文件不可读" : installed === undefined ? "未安装或不可读" :
        installed === source ? "已同步" : "副本待更新",
      imported: imports.some((value) => {
        try {
          const uri = vscode.Uri.parse(value);
          return uri.scheme === "file" &&
            (uri.fsPath === target || uri.fsPath === path.join(context.extensionPath, "extras", name));
        } catch {
          return false;
        }
      }),
      patched: html === undefined ? "无法读取" : injected !== undefined && patch?.includes(injected) ?
        "磁盘补丁已更新" : "未注入当前版本",
    };
  });
}

/** @param {string} value @returns {string} */
function escapeHtml(value) {
  return value.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char] || char));
}

/** @param {import("vscode").ExtensionContext} context @returns {void} */
function showAppearanceStatus(context) {
  const panel = vscode.window.createWebviewPanel("adwcode.appearanceStatus", "Adwaita 外观状态", vscode.ViewColumn.One, { enableScripts: true });
  const render = () => {
    const rows = appearanceStatus(context);
    const loader = vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION);
    const ready = Boolean(loader) && rows.every((row) => row.copied === "已同步" && row.imported && row.patched === "磁盘补丁已更新");
    const nonce = Math.random().toString(36).slice(2);
    panel.webview.html = `<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1">
      <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
      <style>
        body { max-width: 920px; margin: 0 auto; padding: 32px 24px; color: var(--vscode-foreground); font-family: var(--vscode-font-family); line-height: 1.6; }
        h1 { font-size: 26px; margin-bottom: 4px; } h2 { font-size: 17px; }
        .muted { color: var(--vscode-descriptionForeground); }
        .card { background: var(--vscode-editorWidget-background); border: 1px solid var(--vscode-widget-border, transparent); border-radius: 15px; padding: 20px; margin: 20px 0; }
        .table { overflow-x: auto; } table { width: 100%; border-collapse: collapse; text-align: left; }
        th, td { padding: 12px 10px; border-bottom: 1px solid var(--vscode-widget-border, transparent); white-space: nowrap; }
        button { background: var(--vscode-button-background); color: var(--vscode-button-foreground); border: 0; border-radius: 9px; padding: 8px 16px; cursor: pointer; }
        button:hover { background: var(--vscode-button-hoverBackground); } button:focus-visible { outline: 2px solid var(--vscode-focusBorder); outline-offset: 2px; }
      </style></head><body>
      <h1>外观状态</h1><p class="muted">检查文件同步与磁盘补丁，不会修改配置或重载窗口。</p>
      <div class="card"><h2>${ready ? "文件与补丁均已就绪" : "外观文件需要检查"}</h2>
      <p>Custom CSS and JS Loader：${loader ? "已安装" : "未安装"}</p>
      <p>当前窗口是否已加载这些文件：无法直接确认。补丁更新后，待工作结束再手动重载。</p></div>
      <div class="table"><table><thead><tr><th>文件</th><th>安装副本</th><th>加载器配置</th><th>磁盘补丁</th></tr></thead><tbody>
      ${rows.map((row) => `<tr><td>${escapeHtml(row.name)}</td><td>${row.copied}</td><td>${row.imported ? "已加入" : "未加入"}</td><td>${row.patched}</td></tr>`).join("")}
      </tbody></table></div>
      <div class="card"><h2>下一步</h2><p>副本或加载配置需要更新：执行“Adwaita: 安装 GNOME 外观（CSS）”。磁盘补丁需要更新：执行加载器的“Reload Custom CSS and JS”。</p>
      <p class="muted">重载窗口会中断 Codex。此面板不会自动执行上述操作。</p></div>
      <button id="refresh">刷新状态</button>
      <script nonce="${nonce}">const api = acquireVsCodeApi(); document.getElementById('refresh').addEventListener('click', () => api.postMessage('refresh'));</script>
      </body></html>`;
  };
  const listener = panel.webview.onDidReceiveMessage((message) => { if (message === "refresh") render(); });
  panel.onDidDispose(() => listener.dispose());
  render();
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @param {string[]} names
 * @returns {Promise<void>}
 */
async function installCss(context, names) {
  /** @type {string[]} */
  const installed = [];
  try {
    await fs.promises.mkdir(CSS_DIR, { recursive: true });
    for (const name of names) {
      const source = path.join(context.extensionPath, "extras", name);
      const target = path.join(CSS_DIR, name);
      await fs.promises.copyFile(source, target);
      installed.push(target);
    }
  } catch (error) {
    const message = /** @type {Error} */ (error).message;
    vscode.window.showErrorMessage(`Adwaita：无法写入 CSS 文件：${message}`);
    return;
  }

  const uris = installed.map((file) => vscode.Uri.file(file).toString());
  const markers = names.map((name) => CSS_FILES[name]);
  const loader = vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION);
  const state = cssPatchState(markers);

  if (loader) {
    const config = vscode.workspace.getConfiguration("vscode_custom_css");
    /** @type {string[]} */
    const imports = config.get("imports", []);
    // 移除旧版菜单位置脚本，保留其他扩展和用户的加载项。
    const retiredMenu = vscode.Uri.file(path.join(CSS_DIR, "gnome-menu.js")).toString();
    const merged = imports.filter((uri) => uri !== retiredMenu);
    for (const uri of uris) {
      if (!merged.includes(uri)) {
        merged.push(uri);
      }
    }
    if (merged.length !== imports.length || merged.some((uri, index) => uri !== imports[index])) {
      await config.update("imports", merged, vscode.ConfigurationTarget.Global);
    }
    const [action, command, message] =
      state === "not-enabled"
        ? [
            "Enable Custom CSS and JS",
            "extension.installCustomCSS",
            `Adwaita：${names.join("、")} 已配置。请执行一次 “Enable Custom CSS and JS” ` +
              `为 VS Code 打补丁（需要 ${vscode.env.appRoot} 的写权限），然后重载窗口。`,
          ]
        : [
            "Reload Custom CSS and JS",
            "extension.updateCustomCSS",
            `Adwaita：CSS 补丁已过期（例如 VS Code 升级后）。` +
              `请重新加载以应用 ${names.join("、")}。`,
          ];
    const choice = await vscode.window.showInformationMessage(message, action);
    if (choice === action) {
      await vscode.commands.executeCommand(command);
    }
    return;
  }

  await vscode.env.clipboard.writeText(
    `"vscode_custom_css.imports": [\n${uris.map((uri) => `  "${uri}"`).join(",\n")}\n]`
  );
  const open = "打开 CSS 目录";
  const choice = await vscode.window.showInformationMessage(
    `Adwaita：已写入 ${installed.join("、")}，并把 vscode_custom_css.imports 片段复制到剪贴板。` +
      `请安装 “Custom CSS and JS Loader”，把片段粘贴到设置中，执行 “Enable Custom CSS and JS” 后重载窗口。`,
    open
  );
  if (choice === open) {
    await vscode.commands.executeCommand("vscode.open", vscode.Uri.file(CSS_DIR));
  }
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {Promise<void>}
 */
async function applyRecommendedSettings(context) {
  const entries = Object.entries(RECOMMENDED_SETTINGS);
  const preview = entries.map(([key, value]) => `  ${key}: ${JSON.stringify(value)}`).join("\n");
  const apply = "应用";
  const choice = await vscode.window.showInformationMessage(
    `Adwaita 将把 ${entries.length} 项设置改为 GNOME Builder 风格：\n\n${preview}`,
    { modal: true },
    apply
  );
  if (choice !== apply) {
    return;
  }

  // globalState 以 JSON 序列化，undefined 会被丢弃，因此显式记录"原先是否设置过"
  /** @type {Record<string, SettingRecord>} */
  const previous = {};
  for (const [key, value] of entries) {
    const [section, name] = splitSetting(key);
    const config = vscode.workspace.getConfiguration(section);
    const inspected = config.inspect(name);
    const oldValue = inspected ? inspected.globalValue : undefined;
    previous[key] = { wasSet: oldValue !== undefined, value: oldValue === undefined ? null : oldValue };
    await config.update(name, value, vscode.ConfigurationTarget.Global);
  }
  await context.globalState.update("adwcode.previousSettings", previous);

  const reload = "重载窗口";
  const after = await vscode.window.showInformationMessage(
    "Adwaita：推荐设置已应用，重载窗口后全部生效。",
    reload
  );
  if (after === reload) {
    await vscode.commands.executeCommand("workbench.action.reloadWindow");
  }
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {Promise<void>}
 */
async function revertRecommendedSettings(context) {
  /** @type {Record<string, SettingRecord> | undefined} */
  const previous = context.globalState.get("adwcode.previousSettings");
  if (!previous) {
    vscode.window.showInformationMessage("Adwaita：没有可恢复的设置。");
    return;
  }
  for (const [key, record] of Object.entries(previous)) {
    const [section, name] = splitSetting(key);
    const target = record && record.wasSet ? record.value : undefined;
    await vscode.workspace.getConfiguration(section).update(
      name,
      target,
      vscode.ConfigurationTarget.Global
    );
  }
  await context.globalState.update("adwcode.previousSettings", undefined);
  vscode.window.showInformationMessage("Adwaita：已恢复原有设置。");
}

/**
 * @param {string} key
 * @returns {[string, string]}
 */
function splitSetting(key) {
  const index = key.indexOf(".");
  return [key.slice(0, index), key.slice(index + 1)];
}

/** @type {import("vscode").FileSystemWatcher[]} */
let reloadWatchers = [];
/** @type {ReturnType<typeof setTimeout> | undefined} */
let reloadTimer;

/**
 * 重新应用 Custom CSS 并重载窗口（开发时让样式/主题/代码改动立即生效）。
 * @param {import("vscode").ExtensionContext} context
 * @returns {Promise<void>}
 */
async function reloadWithStyles(context) {
  try {
    // 加载器读取安装目录中的副本，只同步用户已安装的样式。
    for (const name of Object.keys(CSS_FILES)) {
      const target = path.join(CSS_DIR, name);
      if (fs.existsSync(target)) {
        await fs.promises.copyFile(path.join(context.extensionPath, "extras", name), target);
      }
    }
  } catch (error) {
    const message = /** @type {Error} */ (error).message;
    vscode.window.showErrorMessage(`Adwaita：无法同步 CSS 文件：${message}`);
    return;
  }
  try {
    // Custom CSS and JS Loader 会把 imports 中的样式重新内联进 workbench.html
    await vscode.commands.executeCommand("extension.updateCustomCSS");
  } catch {
    // 已安装加载器但更新失败时，不把错误当成“未安装”，避免无效重载。
    if (vscode.extensions.getExtension(CUSTOM_CSS_EXTENSION)) {
      vscode.window.showErrorMessage("Adwaita：Custom CSS 更新失败，已取消自动重载。请手动检查加载器。");
      return;
    }
  }
  if (!vscode.workspace.getConfiguration("adwcode").get("autoReload", false)) {
    return;
  }
  await vscode.commands.executeCommand("workbench.action.reloadWindow");
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function scheduleReload(context) {
  if (reloadTimer !== undefined) {
    clearTimeout(reloadTimer);
  }
  reloadTimer = setTimeout(() => {
    reloadTimer = undefined;
    reloadWithStyles(context).catch(() => undefined);
  }, 1500);
}

/**
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function startReloadWatchers(context) {
  if (reloadWatchers.length > 0) {
    return;
  }
  for (const pattern of ["extension/extension.js", "extras/*.css", "extras/*.js", "themes/*.json", "package.json"]) {
    const watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(context.extensionPath, pattern)
    );
    watcher.onDidChange(() => scheduleReload(context));
    watcher.onDidCreate(() => scheduleReload(context));
    watcher.onDidDelete(() => scheduleReload(context));
    reloadWatchers.push(watcher);
  }
}

/** @returns {void} */
function stopReloadWatchers() {
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
 * 按 `adwcode.autoReload` 设置启停文件监视。
 * @param {import("vscode").ExtensionContext} context
 * @returns {void}
 */
function syncReloadWatchers(context) {
  const enabled = /** @type {boolean} */ (
    vscode.workspace.getConfiguration("adwcode").get("autoReload", false)
  );
  if (enabled) {
    startReloadWatchers(context);
  } else {
    stopReloadWatchers();
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
    vscode.commands.registerCommand("adwcode.appearanceStatus", () => showAppearanceStatus(context)),
    vscode.commands.registerCommand("adwcode.syncAccent", () => syncAccent(true)),
    vscode.commands.registerCommand("adwcode.installGnomeLook", () =>
      installCss(context, ["gnome-look.css", "controls-close-only.css"])
    ),
    vscode.commands.registerCommand("adwcode.closeOnlyControls", () =>
      installCss(context, ["controls-close-only.css"])
    ),
    vscode.commands.registerCommand("adwcode.applyRecommendedSettings", () =>
      applyRecommendedSettings(context)
    ),
    vscode.commands.registerCommand("adwcode.revertRecommendedSettings", () =>
      revertRecommendedSettings(context)
    ),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("adwcode.autoReload")) {
        syncReloadWatchers(context);
      }
      if (
        event.affectsConfiguration("workbench.colorTheme") ||
        event.affectsConfiguration("workbench.preferredDarkColorTheme") ||
        event.affectsConfiguration("workbench.preferredLightColorTheme") ||
        event.affectsConfiguration("adwcode.autoAccent")
      ) {
        syncAccent().catch(() => undefined);
      }
    }),
    { dispose: stopAccentMonitor },
    { dispose: stopReloadWatchers }
  );

  syncAccent().catch(() => undefined);
  syncReloadWatchers(context);
}

/** @returns {void} */
function deactivate() {
  extensionContext = undefined;
  stopAccentMonitor();
  stopReloadWatchers();
}

/** @type {{ activate: typeof activate, deactivate: typeof deactivate }} */
module.exports = { activate, deactivate };
