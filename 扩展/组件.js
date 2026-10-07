// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// @ts-check
/**
 * @param {typeof import("os")} os
 * @param {typeof import("path")} path
 * @param {Record<string, string | undefined>} 环境
 */
function 创建组件(os, path, 环境) {
const 加载器标识 = "be5invis.vscode-custom-css";
const 旧配置目录 = path.join(os.homedir(), ".config", "adwcode");
const 配置根目录 = 环境.XDG_CONFIG_HOME;
const 安装目录 = 配置根目录 && path.isAbsolute(配置根目录)
  ? path.join(配置根目录, "adwcode") : 旧配置目录;

/**
 * 附加外观/ 中的外观样式与状态脚本，以及各自在补丁 HTML 中的识别标记。
 * @type {Record<string, string>}
 */
const 组件标记 = {
  "GNOME外观.css": "--vscode-cornerRadius-small",
  "仅关闭窗口控件.css": "window-max-restore",
  "GNOME字体.css": "--adwcode-ui-font",
  "窗口状态.js": "adwcode.windowState",
};

// 仅用于清理旧版引用，不提供旧文件名的功能入口。
const 旧加载文件 = ["gnome-look.css", "controls-close-only.css", "gnome-fonts.css", "window-state.js", "gnome-menu.js"];


return { 加载器标识, 安装目录, 组件标记, 旧配置目录, 旧加载文件 };
}
module.exports = { 创建组件 };
