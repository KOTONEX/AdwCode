// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

function 注册命令(
    vscode: typeof import("vscode"),
    服务: ReturnType<typeof import("./外观服务").创建外观服务>,
    context: import("vscode").ExtensionContext,
) {
    const {
        组件标记,
        读取系统字体,
        显示外观安装状态,
        排队外观操作,
        安装样式,
        选择外观组件,
        执行外观事务,
    } = 服务;
    context.subscriptions.push(
        vscode.commands.registerCommand("adwcode.查看外观安装状态", () =>
            排队外观操作(async () => {
                await 读取系统字体();
                显示外观安装状态(context);
            }),
        ),
        vscode.commands.registerCommand("adwcode.安装GNOME外观", () =>
            排队外观操作(() => 安装样式(context, Object.keys(组件标记))),
        ),
        vscode.commands.registerCommand("adwcode.选择外观组件", () =>
            排队外观操作(() => 选择外观组件(context)),
        ),
        vscode.commands.registerCommand("adwcode.移除外观引用", () =>
            排队外观操作(() => 执行外观事务(context, [], true)),
        ),
        vscode.commands.registerCommand("adwcode.安装仅关闭窗口控件", () =>
            排队外观操作(() => 安装样式(context, ["仅关闭窗口控件.css"])),
        ),
    );
}

export { 注册命令 };
