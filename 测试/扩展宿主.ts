// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
import * as path from "path";
import * as fs from "fs";
import * as vm from "vm";
import { 创建外观服务 } from "../扩展/外观服务";
export interface 测试依赖 {
    require(name: string): unknown;
    process: { platform: string; env?: Record<string, string | undefined> };
}
// 故意不完整的模拟模块只在这个测试边界转成真实 API 类型；业务模块始终使用严格类型。
export function 创建测试服务(依赖: 测试依赖) {
    return 创建外观服务({
        vscode: 依赖.require("vscode") as typeof import("vscode"),
        fs: 依赖.require("fs") as typeof import("fs"),
        os: 依赖.require("os") as typeof import("os"),
        path,
        execFile: (
            依赖.require("child_process") as typeof import("child_process")
        ).execFile,
        环境: 依赖.process.env || {},
    });
}
export function 创建测试上下文(
    extensionPath: string,
): import("vscode").ExtensionContext {
    return {
        extensionPath,
        subscriptions: [],
        globalState: { get: () => undefined, update: async () => {} },
    };
}
export function 加载测试入口(依赖: 测试依赖): typeof import("../扩展/扩展") {
    const 文件 = path.join(__dirname, "../扩展/扩展.js");
    const 导出 = {};
    const 上下文 = {
        module: { exports: 导出 },
        exports: 导出,
        process: 依赖.process,
        require(name: string) {
            return name.startsWith("./")
                ? require(path.resolve(path.dirname(文件), name))
                : 依赖.require(name);
        },
    };
    vm.runInNewContext(fs.readFileSync(文件, "utf8"), 上下文, {
        filename: 文件,
    });
    return 上下文.module.exports as typeof import("../扩展/扩展");
}
