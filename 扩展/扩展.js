// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// @ts-check
const vscode = /** @type {typeof import("vscode")} */ (require("vscode"));
const { execFile } = /** @type {typeof import("child_process")} */ (require("child_process"));
const fs = /** @type {typeof import("fs")} */ (require("fs"));
const os = /** @type {typeof import("os")} */ (require("os"));
const path = /** @type {typeof import("path")} */ (require("path"));
const 服务 = require("./外观服务").创建外观服务({ vscode, execFile, fs, os, path, 环境: process.env || {} });
/** @param {import("vscode").ExtensionContext} context */
function activate(context) { require("./命令").注册命令(vscode, 服务, context); }
module.exports = { activate };
