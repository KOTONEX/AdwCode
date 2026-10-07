// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// @ts-check
/**
 * @param {typeof import("vscode")} vscode
 * @param {typeof import("path")} path
 * @param {ReturnType<typeof import("./组件").创建组件>} 组件
 */
function 创建引用(vscode, path, 组件) {
const { 安装目录, 组件标记, 旧配置目录, 旧加载文件 } = 组件;
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
    if (![...Object.keys(组件标记), ...旧加载文件].includes(name)) return undefined;
    // 旧目录仅参与失效引用清理，不作为加载入口。
    const folders = [安装目录, 旧配置目录, path.join(context.extensionPath, "附加外观")];
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
        merged.push(vscode.Uri.file(path.join(安装目录, name)).toString());
        added.add(name);
      }
    } else {
      merged.push(value);
    }
  }
  for (const name of names) {
    if (!added.has(name)) {
      merged.push(vscode.Uri.file(path.join(安装目录, name)).toString());
      added.add(name);
    }
  }
  return merged;
}
/** @param {unknown} 值 @returns {string[]} */
function 校验加载引用(值) {
  if (!Array.isArray(值) || !值.every(项 => typeof 项 === "string")) {
    throw Error("vscode_custom_css.imports 必须是字符串数组");
  }
  return 值;
}

return { 识别加载文件, 合并加载引用, 校验加载引用 };
}
module.exports = { 创建引用 };
