// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
import * as 外观服务模块 from "./外观服务";
import * as 命令模块 from "./命令";
import * as vscode from "vscode";
import { execFile } from "child_process";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
const 服务 = 外观服务模块.创建外观服务({
    vscode,
    execFile,
    fs,
    os,
    path,
    环境: process.env || {},
});

function activate(context: import("vscode").ExtensionContext) {
    命令模块.注册命令(vscode, 服务, context);
}
export { activate };
