// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
import * as 事务模块 from "./事务";
import * as 状态模块 from "./状态";
import * as 引用模块 from "./引用";
import * as 字体模块 from "./字体";
import * as 组件模块 from "./组件";

function 创建外观服务(依赖: {
    vscode: typeof import("vscode");
    fs: typeof import("fs");
    os: typeof import("os");
    path: typeof import("path");
    execFile: typeof import("child_process").execFile;
    环境: Record<string, string | undefined>;
}) {
    const { vscode, fs, os, path, execFile, 环境 } = 依赖;
    const 组件 = 组件模块.创建组件(os, path, 环境);
    const 字体 = 字体模块.创建字体(vscode, execFile, fs, path);
    const 引用 = 引用模块.创建引用(vscode, path, 组件);
    const 状态 = 状态模块.创建状态(vscode, fs, path, 组件, 字体, 引用);
    const 事务 = 事务模块.创建事务(vscode, fs, path, 组件, 字体, 引用, 状态);
    return { ...组件, ...字体, ...引用, ...状态, ...事务 };
}
export { 创建外观服务 };
