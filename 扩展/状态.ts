// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
type 补丁状态 = "unknown" | "not-enabled" | "enabled" | "stale";

function 创建状态(
    vscode: typeof import("vscode"),
    fs: typeof import("fs"),
    path: typeof import("path"),
    组件: ReturnType<typeof import("./组件").创建组件>,
    字体: ReturnType<typeof import("./字体").创建字体>,
    引用: ReturnType<typeof import("./引用").创建引用>,
) {
    const { 安装目录, 组件标记, 加载器标识 } = 组件;
    const { 样式源码 } = 字体;
    const { 识别加载文件, 校验加载引用 } = 引用;

    function 工作台HTML路径(): string | undefined {
        const candidates = [
            path.join(
                vscode.env.appRoot,
                "out",
                "vs",
                "code",
                "electron-browser",
                "workbench",
                "workbench.esm.html",
            ),
            path.join(
                vscode.env.appRoot,
                "out",
                "vs",
                "code",
                "electron-browser",
                "workbench",
                "workbench.html",
            ),
            path.join(
                vscode.env.appRoot,
                "out",
                "vs",
                "code",
                "electron-sandbox",
                "workbench",
                "workbench.html",
            ),
        ];
        return candidates.find((candidate) => fs.existsSync(candidate));
    }

    function 样式补丁状态(markers: string[]): 补丁状态 {
        const html = 工作台HTML路径();
        if (!html) {
            return "unknown";
        }
        let content;
        try {
            content = fs.readFileSync(html, "utf8");
        } catch (error) {
            return "unknown";
        }
        const patch = content.match(
            /<!-- !! VSCODE-CUSTOM-CSS-START !! -->([\s\S]*?)<!-- !! VSCODE-CUSTOM-CSS-END !! -->/,
        )?.[1];
        if (patch === undefined) {
            return "not-enabled";
        }
        return markers.every((marker) => patch.includes(marker))
            ? "enabled"
            : "stale";
    }

    function 外观安装状态(context: import("vscode").ExtensionContext) {
        const imports = 校验加载引用(
            vscode.workspace
                .getConfiguration("vscode_custom_css")
                .get("imports", []),
        );
        const htmlPath = 工作台HTML路径();
        let html;
        try {
            html = htmlPath ? fs.readFileSync(htmlPath, "utf8") : undefined;
        } catch {
            html = undefined;
        }
        const patch = html?.match(
            /<!-- !! VSCODE-CUSTOM-CSS-START !! -->([\s\S]*?)<!-- !! VSCODE-CUSTOM-CSS-END !! -->/,
        )?.[1];
        return Object.keys(组件标记).map((name) => {
            const target = path.join(安装目录, name);
            let source;
            let installed;
            try {
                source = 样式源码(context, name);
            } catch {
                /* 单独报告 */
            }
            try {
                installed = fs.readFileSync(target, "utf8");
            } catch {
                /* 单独报告 */
            }
            const injected =
                source === undefined
                    ? undefined
                    : name.endsWith(".js")
                      ? `<script>${source}</script>`
                      : `<style>${source}</style>`;
            const importCount = imports.filter(
                (value) => 识别加载文件(context, value) === name,
            ).length;
            const patchCount = [
                ...(patch || "").matchAll(
                    /<(?:style|script)>([\s\S]*?)<\/(?:style|script)>/g,
                ),
            ].filter(
                (match) =>
                    match[1].includes(组件标记[name]) || match[0] === injected,
            ).length;
            return {
                name,
                copied:
                    source === undefined
                        ? "源文件不可读"
                        : installed === undefined
                          ? "未安装或不可读"
                          : installed === source
                            ? "已同步"
                            : "副本待更新",
                imported: importCount > 0,
                importCount,
                patched:
                    html === undefined
                        ? "无法读取"
                        : patchCount > 1
                          ? "重复注入，补丁待更新"
                          : injected !== undefined && patch?.includes(injected)
                            ? "磁盘补丁已更新"
                            : "未注入当前版本",
            };
        });
    }

    let 外观状态输出通道: import("vscode").OutputChannel | undefined;

    function 外观状态通道(
        context: import("vscode").ExtensionContext,
    ): import("vscode").OutputChannel {
        if (外观状态输出通道 === undefined) {
            const channel =
                vscode.window.createOutputChannel("AdwCode 外观状态");
            外观状态输出通道 = channel;
            context.subscriptions.push({
                dispose: () => {
                    外观状态输出通道 = undefined;
                    channel.dispose();
                },
            });
        }
        return 外观状态输出通道;
    }

    function 显示外观安装状态(
        context: import("vscode").ExtensionContext,
    ): void {
        const rows = 外观安装状态(context);
        const loader = vscode.extensions.getExtension(加载器标识);
        const ready =
            Boolean(loader) &&
            rows.every(
                (row) =>
                    row.copied === "已同步" &&
                    row.importCount === 1 &&
                    row.patched === "磁盘补丁已更新",
            );
        const labels: Record<string, string> = {
            "GNOME外观.css": "工作台外观",
            "仅关闭窗口控件.css": "窗口按钮",
            "GNOME字体.css": "界面字体",
            "窗口状态.js": "窗口状态",
        };
        const lines = [
            "外观状态",
            "========",
            "只检查安装文件与磁盘补丁，不修改配置或重载窗口。再次执行“AdwCode: 查看外观安装状态”即刷新。",
            "",
            `安装准备：${ready ? "文件与补丁均已就绪" : "外观文件需要检查"}`,
            `- 加载器（Custom CSS and JS Loader）：${loader ? "已安装" : "未安装"}`,
            "- 当前窗口：无法直接确认是否已加载；磁盘补丁状态与窗口显示分别检查。",
            "",
            "外观组件：",
        ];
        for (const row of rows) {
            lines.push(
                `[${labels[row.name] || row.name}] ${row.name}`,
                `- 安装副本：${row.copied}`,
                `- 加载器配置：${row.importCount > 1 ? `重复引用（${row.importCount} 项），请重新安装外观` : row.imported ? "已加入" : "未加入"}`,
                `- 磁盘补丁：${row.patched}`,
            );
        }
        let step = 0;
        lines.push("", "下一步：");
        if (!loader) lines.push(`${++step}. 安装 Custom CSS and JS Loader。`);
        lines.push(
            `${++step}. 副本或加载配置需要更新时，执行“AdwCode: 安装 GNOME 外观（CSS）”。`,
            `${++step}. 磁盘补丁需要更新时，首次执行加载器的“Enable Custom CSS and JS”；已启用时执行“Reload Custom CSS and JS”。`,
            `${++step}. 保存工作后手动重载窗口，再检查实际外观。重载可能中断扩展会话或调试任务。`,
            "",
            "此输出只读取状态，不会自动执行这些操作。",
            "",
        );
        const channel = 外观状态通道(context);
        channel.replace(lines.join("\n"));
        channel.show();
    }

    return { 样式补丁状态, 外观安装状态, 显示外观安装状态 };
}
export { 创建状态 };
