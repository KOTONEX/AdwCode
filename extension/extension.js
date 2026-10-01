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
  "editor.fontFamily": "Adwaita Mono, monospace",
  "editor.renderLineHighlight": "none",
  "editor.minimap.enabled": false,
  "editor.guides.indentation": true,
  "editor.stickyScroll.enabled": false,
  "editor.smoothScrolling": true,
  "breadcrumbs.enabled": false,
  "scm.diffDecorations": "none",
  "window.commandCenter": false,
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

  if (state === "enabled") {
    const reload = "Reload Custom CSS and JS";
    const choice = await vscode.window.showInformationMessage(
      `Adwaita：${names.join("、")} 已生效。${CSS_DIR} 中的文件刚刚更新——` +
        `执行加载器的 Reload Custom CSS and JS 以应用当前版本。`,
      reload
    );
    if (choice === reload) {
      await vscode.commands.executeCommand("extension.updateCustomCSS");
    }
    return;
  }

  if (loader) {
    const config = vscode.workspace.getConfiguration("vscode_custom_css");
    /** @type {string[]} */
    const imports = config.get("imports", []);
    const merged = [...imports];
    for (const uri of uris) {
      if (!merged.includes(uri)) {
        merged.push(uri);
      }
    }
    if (merged.length !== imports.length) {
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
      if (
        event.affectsConfiguration("workbench.colorTheme") ||
        event.affectsConfiguration("workbench.preferredDarkColorTheme") ||
        event.affectsConfiguration("workbench.preferredLightColorTheme") ||
        event.affectsConfiguration("adwcode.autoAccent")
      ) {
        syncAccent().catch(() => undefined);
      }
    }),
    { dispose: stopAccentMonitor }
  );

  syncAccent().catch(() => undefined);
}

/** @returns {void} */
function deactivate() {
  extensionContext = undefined;
  stopAccentMonitor();
}

/** @type {{ activate: typeof activate, deactivate: typeof deactivate }} */
module.exports = { activate, deactivate };
