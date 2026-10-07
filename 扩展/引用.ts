// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

function 创建引用(
    vscode: typeof import("vscode"),
    path: typeof import("path"),
    组件: ReturnType<typeof import("./组件").创建组件>,
) {
    const { 安装目录, 组件标记, 旧配置目录, 旧加载文件 } = 组件;

    function 识别加载文件(
        context: import("vscode").ExtensionContext,
        value: string,
    ): string | undefined {
        try {
            const uri = vscode.Uri.parse(value);
            if (uri.scheme !== "file") return undefined;
            const file = path.resolve(uri.fsPath);
            const name = path.basename(file);
            if (![...Object.keys(组件标记), ...旧加载文件].includes(name))
                return undefined;
            // 旧目录仅参与失效引用清理，不作为加载入口。
            const folders = [
                安装目录,
                旧配置目录,
                path.join(context.extensionPath, "附加外观"),
                path.join(
                    context.extensionPath,
                    "builddir",
                    "脚本",
                    "附加外观",
                ),
            ];
            if (旧加载文件.includes(name))
                folders.push(path.join(context.extensionPath, "extras"));
            return folders.some((folder) => file === path.resolve(folder, name))
                ? name
                : undefined;
        } catch {
            return undefined;
        }
    }

    function 合并加载引用(
        context: import("vscode").ExtensionContext,
        imports: string[],
        names: string[],
    ): string[] {
        const merged: string[] = [];
        const added = new Set<string>();
        for (const value of imports) {
            const name = 识别加载文件(context, value);
            if (name && 旧加载文件.includes(name)) continue;
            if (name && names.includes(name)) {
                if (!added.has(name)) {
                    merged.push(
                        vscode.Uri.file(path.join(安装目录, name)).toString(),
                    );
                    added.add(name);
                }
            } else {
                merged.push(value);
            }
        }
        for (const name of names) {
            if (!added.has(name)) {
                merged.push(
                    vscode.Uri.file(path.join(安装目录, name)).toString(),
                );
                added.add(name);
            }
        }
        return merged;
    }

    function 校验加载引用(值: unknown): string[] {
        if (!Array.isArray(值) || !值.every((项) => typeof 项 === "string")) {
            throw Error("vscode_custom_css.imports 必须是字符串数组");
        }
        return 值;
    }

    return { 识别加载文件, 合并加载引用, 校验加载引用 };
}
export { 创建引用 };
