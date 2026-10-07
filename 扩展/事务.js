// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// @ts-check
/**
 * @param {typeof import("vscode")} vscode
 * @param {typeof import("fs")} fs
 * @param {typeof import("path")} path
 * @param {ReturnType<typeof import("./组件").创建组件>} 组件
 * @param {ReturnType<typeof import("./字体").创建字体>} 字体
 * @param {ReturnType<typeof import("./引用").创建引用>} 引用
 * @param {ReturnType<typeof import("./状态").创建状态>} 状态
 */
function 创建事务(vscode, fs, path, 组件, 字体, 引用, 状态) {
const { 安装目录, 组件标记, 加载器标识 } = 组件;
const { 读取系统字体, 样式源码 } = 字体;
const { 识别加载文件, 合并加载引用, 校验加载引用 } = 引用;
const { 样式补丁状态 } = 状态;
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
  if (names.some(name => !Object.hasOwn(组件标记, name))) throw Error("未知外观组件");
  await 读取系统字体();
  const loader = vscode.extensions.getExtension(加载器标识);
  const config = vscode.workspace.getConfiguration("vscode_custom_css");
  const inspected = config.inspect("imports");
  const 原全局配置 = inspected?.globalValue;
  const imports = 校验加载引用(原全局配置 ?? inspected?.defaultValue ?? []);
  const merged = 合并加载引用(context, 替换全部 ? imports.filter(value => !识别加载文件(context, value)) : imports, names);
  /** @type {{target: string, temporary: string, before: string | undefined}[]} */
  const 文件快照 = [];
  let 配置更新开始 = false;
  try {
    if (names.length) await fs.promises.mkdir(安装目录, { recursive: true, mode: 0o700 });
    // 先读取全部源文件和旧副本，再写暂存文件，最后逐项替换。
    const 内容 = names.map(name => ({ name, data: 样式源码(context, name) }));
    for (const {name} of 内容) {
      const target = path.join(安装目录, name);
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
  const installed = names.map(name => path.join(安装目录, name));
  const markers = names.map(name => 组件标记[name]);
  const state = 样式补丁状态(markers);
  if (替换全部 && !names.length) {
    if (!loader) await vscode.env.clipboard.writeText('"vscode_custom_css.imports": ' + JSON.stringify(merged, null, 2));
    await vscode.window.showInformationMessage("AdwCode：已移除用户级自有外观引用，保留其他加载项和磁盘副本。工作区覆盖需手动移除；请手动更新加载器并重载窗口。");
    return;
  }
  if (loader) {
    const activeConfig = vscode.workspace.getConfiguration("vscode_custom_css");
    const activeScope = activeConfig.inspect("imports");
    const effective = 校验加载引用(activeConfig.get("imports", []));
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
    `"vscode_custom_css.imports": ${JSON.stringify(merged, null, 2)}`
  );
  const open = "打开外观目录";
  const choice = await vscode.window.showInformationMessage(
    `AdwCode：已写入 ${installed.join("、")}，并把 vscode_custom_css.imports 片段复制到剪贴板。` +
      `请安装 “Custom CSS and JS Loader”，把片段粘贴到设置中，执行 “Enable Custom CSS and JS” 后重载窗口。`,
    open
  );
  if (choice === open) {
    await vscode.commands.executeCommand("vscode.open", vscode.Uri.file(安装目录));
  }
}

/** @param {import("vscode").ExtensionContext} context @returns {Promise<void>} */
async function 选择外观组件(context) {
  const 当前 = 校验加载引用(vscode.workspace.getConfiguration("vscode_custom_css").get("imports", []));
  const 选项 = Object.keys(组件标记).map(name => ({ label: name, picked: 当前.some(value => 识别加载文件(context, value) === name) }));
  const 选择 = await vscode.window.showQuickPick(选项, { canPickMany: true, placeHolder: "选择要加载的外观组件；清空选择可移除全部自有引用" });
  if (选择 !== undefined) await 执行外观事务(context, 选择.map(item => item.label), true);
}

return { 安装样式, 执行外观事务, 排队外观操作, 选择外观组件 };
}
module.exports = { 创建事务 };
