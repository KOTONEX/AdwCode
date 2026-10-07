// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

function 创建字体(
    vscode: typeof import("vscode"),
    execFile: typeof import("child_process").execFile,
    fs: typeof import("fs"),
    path: typeof import("path"),
) {
    let systemFonts: { ui?: string; mono?: string } = {};

    function 解析Pango字体(description: string): string | undefined {
        const value = description
            .trim()
            .replace(/^'|'$/g, "")
            .replace(/\\(['\\])/g, "$1");
        if (!/\s+\d+(?:\.\d+)?$/.test(value)) return undefined;
        return (
            value
                .replace(/\s+\d+(?:\.\d+)?$/, "")
                .replace(
                    /(?:\s+(?:Bold|Semi-Bold|Semibold|Italic|Oblique|Regular|Medium|Light))+$/i,
                    "",
                )
                .trim() || undefined
        );
    }

    function 引用字体名称(value: string): string {
        return (
            '"' +
            value.replace(
                /[\\"\x00-\x1f<>]/g,
                (char) => `\\${char.charCodeAt(0).toString(16)} `,
            ) +
            '"'
        );
    }

    async function 读取系统字体() {
        const read = (key: string) =>
            new Promise<string | undefined>((resolve) => {
                execFile(
                    "gsettings",
                    ["get", "org.gnome.desktop.interface", key],
                    { timeout: 5000 },
                    (error, stdout) =>
                        resolve(
                            error ? undefined : 解析Pango字体(String(stdout)),
                        ),
                );
            });
        const [ui, mono] = await Promise.all([
            read("font-name"),
            read("monospace-font-name"),
        ]);
        systemFonts = { ui, mono };
    }

    function 界面字体栈(): string {
        const configured = vscode.workspace
            .getConfiguration("adwcode")
            .get("界面字体", "");
        const family =
            typeof configured === "string" && configured.trim()
                ? configured.trim()
                : systemFonts.ui;
        return (
            (family ? `${引用字体名称(family)}, ` : "") +
            '"Adwaita Sans", "Cantarell", system-ui, sans-serif'
        );
    }

    function 样式源码(
        context: import("vscode").ExtensionContext,
        name: string,
    ): string {
        const source = fs.readFileSync(
            path.join(
                context.extensionPath,
                ...(name === "窗口状态.js"
                    ? ["builddir", "脚本", "附加外观", name]
                    : ["附加外观", name]),
            ),
            "utf8",
        );
        if (name !== "GNOME字体.css") return source;
        return (
            source +
            `\n:root, .monaco-workbench { --adwcode-ui-font: ${界面字体栈()}; }\n`
        );
    }

    return { 解析Pango字体, 引用字体名称, 读取系统字体, 样式源码 };
}
export { 创建字体 };
