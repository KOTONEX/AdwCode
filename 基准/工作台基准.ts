// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 仅连接 Rust 工作台基准 创建的独立进程；CSS 注入只修改该窗口的 DOM。
import * as fs from "fs";
import * as path from "path";
import * as assert from "assert/strict";
import { performance as 节点计时 } from "perf_hooks";
const { chromium } = require(
    path.resolve(process.argv[3]),
) as typeof import("playwright");
interface 工作台状态 {
    pid: number;
    profile: string;
    directory: string;
    port: number;
    referenceCss?: string;
    scenario?: string;
}
interface 主题信息 {
    name: string;
    type: string;
}
interface 外观对照 {
    active: string[][];
    inactive: string[][];
    activeBody: string[];
    inactiveBody: string[];
}
interface 采样结果 {
    wall_ms: number;
    iterations: number;
    frames: number[];
    nodes: number;
    visibleLineStates?: number;
}
const state = JSON.parse(
    fs.readFileSync(process.argv[2], "utf8"),
) as 工作台状态;
const output = path.resolve(process.argv[4]);
const runs = Number(process.argv[5] || 6);
const selectorsOnly = process.argv[6] === "true";
const root = path.resolve(__dirname, "../../..");
const ownProcess = fs.readFileSync(`/proc/${state.pid}/cmdline`, "utf8");
assert.ok(
    ownProcess.includes(state.profile) && ownProcess.includes(state.directory),
    "只能连接本次启动的测试进程",
);
const css = ["GNOME外观.css", "仅关闭窗口控件.css", "GNOME字体.css"]
    .map((name) => fs.readFileSync(path.join(root, "附加外观", name), "utf8"))
    .join("\n");
const windowState = fs.readFileSync(
    path.join(root, "builddir/脚本/附加外观/窗口状态.js"),
    "utf8",
);
const withoutInactive = css.replace(
    /[^{}]*adwcode-window-inactive[^{}]*\{[^{}]*\}/g,
    "",
);
assert.ok(!withoutInactive.includes(".adwcode-window-inactive"));
const variants = [
    { name: "native", enabled: false, css },
    { name: "full", enabled: true, css },
];
if (state.referenceCss)
    variants.push({
        name: "reference",
        enabled: true,
        css:
            fs.readFileSync(state.referenceCss, "utf8") +
            "\n" +
            ["仅关闭窗口控件.css", "GNOME字体.css"]
                .map((name) =>
                    fs.readFileSync(path.join(root, "附加外观", name), "utf8"),
                )
                .join("\n"),
    });
if (selectorsOnly)
    variants.push({
        name: "without-inactive-selectors",
        enabled: true,
        css: withoutInactive,
    });
const themes = ["light", "dark"].map(
    (mode) =>
        JSON.parse(
            fs.readFileSync(
                path.join(
                    root,
                    "主题",
                    `adwcode-${mode === "light" ? "浅色" : "深色"}.json`,
                ),
                "utf8",
            ),
        ) as 主题信息,
);
const 等待 = (ms: number) =>
    new Promise<void>((resolve) => setTimeout(resolve, ms));
async function 运行基准() {
    let browser;
    const deadline = Date.now() + 30000;
    while (!browser && Date.now() < deadline) {
        try {
            browser = await chromium.connectOverCDP(
                `http://127.0.0.1:${state.port}`,
                { timeout: 1000 },
            );
        } catch {
            await 等待(300);
        }
    }
    assert.ok(browser, "独立窗口的调试端口未就绪");
    try {
        let pages: import("playwright").页面[] = [];
        for (let attempt = 0; attempt < 60 && pages.length === 0; attempt++) {
            pages = browser
                .contexts()
                .flatMap((context) => context.pages())
                .filter((page) => page.url().startsWith("vscode-file:"));
            if (pages.length === 0) await 等待(300);
        }
        assert.equal(pages.length, 1, "独立工作台页面必须唯一");
        const page = pages[0];
        await page.waitForSelector(".monaco-editor .view-lines", {
            timeout: 30000,
        });
        assert.ok((await page.title()).includes("sample.py"));
        await page.waitForTimeout(1500);
        if (await page.locator(".onboarding-a-close-btn").isVisible())
            await page.locator(".onboarding-a-close-btn").click();
        // 虚拟行容器覆盖整份文档，不能让 click 的自动 scrollIntoView 移动它。
        const editorBox = await page
            .locator(".monaco-editor")
            .first()
            .boundingBox();
        assert.ok(editorBox && editorBox.height > 120);
        await page.mouse.click(editorBox.x + 160, editorBox.y + 100);
        await page.keyboard.press("Control+Home");
        // 安装目录中的既有补丁会被新窗口读取，先从测试 DOM 移除项目样式。
        const removedStyles = await page.evaluate(() => {
            const styles = [...document.querySelectorAll("style")].filter(
                (style) =>
                    /^\/\* SPDX-License-Identifier: AGPL-3\.0-or-later(?: OR LicenseRef-MulanPubL-2\.0(?:-or-later)?)? \*\//.test(
                        style.textContent.trim(),
                    ) && style.textContent.includes("AdwCode 贡献者"),
            );
            const sizes = styles.map((style) => style.textContent.length);
            styles.forEach((style) => style.remove());
            return sizes;
        });
        await page.addStyleTag({ content: css });
        // addStyleTag 的节点位于 head；直接按内容识别，避免误标记主题服务的样式。
        await page.evaluate((css) => {
            const style = [...document.querySelectorAll("style")].find(
                (style) => style.textContent === css,
            );
            if (!style) throw Error("未找到基准样式");
            style.id = "adwcode-performance-style";
        }, css);
        await page.addScriptTag({ content: windowState });
        const session = await page.context().newCDPSession(page);
        await session.send("Performance.enable");
        const layout = await page.evaluate(() => {
            const 祖先 = (element: Element | null) => {
                const result = [];
                for (
                    ;
                    element && result.length < 8;
                    element = element.parentElement
                )
                    result.push(element.className);
                return result;
            };
            return {
                titlebar: 祖先(
                    document.querySelector<HTMLElement>(".part.titlebar")!,
                ),
                editor: 祖先(document.querySelector(".part.editor")),
            };
        });
        const results: object[] = [];
        const metadata = {
            schema: 1,
            version: (
                JSON.parse(
                    fs.readFileSync(path.join(root, "package.json"), "utf8"),
                ) as { version: string }
            ).version,
            browser: await browser.version(),
            method: {
                runs,
                warmup: 1,
                layout,
                fixture: selectorsOnly
                    ? "6000 行 Python 文件、200 个目录文件；仅运行非活动状态规则对照"
                    : "6000 行 Python 文件、200 个目录文件；压力场景额外添加 1000 个非虚拟行",
                limits: "独立真实工作台；CSS 开关对比使用相同主题与产品图标；并非完整启动或插件组合基准",
                removedInjectedStyleLengths: removedStyles,
                selectorsOnly,
                reference: Boolean(state.referenceCss),
                stateSync:
                    "每次切换后等待一个微任务，使标题栏观察器完成状态同步；各样式均运行该脚本",
                scenario: state.scenario || "all",
                frames: "串行 CDP 输入后的 RAF 采样间隔，不是应用帧率或掉帧数据",
            },
            appearanceChecks: [] as object[],
            lifecycleChecks: undefined as
                { lateStartup: boolean; disposed: boolean } | undefined,
        };
        const 保存 = (complete: boolean) =>
            fs.writeFileSync(
                output,
                JSON.stringify({ ...metadata, complete, results }, null, 2) +
                    "\n",
            );
        保存(false);
        const 读取指标 = async () =>
            Object.fromEntries(
                (await session.send("Performance.getMetrics")).metrics.map(
                    (metric) => [metric.name, metric.value],
                ),
            );
        const 验证外观 = async () => {
            const captures: Record<string, 外观对照> = {};
            for (const variant of variants.filter((item) =>
                ["full", "reference"].includes(item.name),
            )) {
                captures[variant.name] = await page.evaluate(
                    async (variant) => {
                        const style = document.querySelector<HTMLStyleElement>(
                            "#adwcode-performance-style",
                        )!;
                        style.textContent = variant.css;
                        style.sheet!.disabled = false;
                        const titlebar =
                            document.querySelector<HTMLElement>(
                                ".part.titlebar",
                            )!;
                        const read = () => {
                            const elements = document.querySelectorAll(
                                ".part.editor .tabs-container > .tab, .part.editor .editor-actions .action-label, .part.sidebar > .title .action-label, .part.auxiliarybar > .title .action-label, .part.panel > .title .action-label, .part.activitybar .action-label",
                            );
                            return [...elements].map((element) => {
                                const computed = getComputedStyle(element);
                                return [
                                    computed.color,
                                    computed.backgroundColor,
                                    computed.opacity,
                                ];
                            });
                        };
                        const body = () =>
                            [
                                ...document.querySelectorAll(
                                    ".monaco-editor .view-line span",
                                ),
                            ].map((element) => getComputedStyle(element).color);
                        titlebar.classList.remove("inactive");
                        await Promise.resolve();
                        const active = read(),
                            activeBody = body();
                        titlebar.classList.add("inactive");
                        await Promise.resolve();
                        return {
                            active,
                            inactive: read(),
                            activeBody,
                            inactiveBody: body(),
                        };
                    },
                    variant,
                );
            }
            assert.deepEqual(
                captures.full.activeBody,
                captures.full.inactiveBody,
                "正文颜色不得随窗口弱化",
            );
            assert.notDeepEqual(
                captures.full.active,
                captures.full.inactive,
                "非活动视觉必须实际生效",
            );
            if (captures.reference)
                assert.deepEqual(
                    captures.full,
                    captures.reference,
                    "导航和标签状态必须与原样式一致",
                );
            // 重复注入脚本应先释放旧观察器，并保持当前状态。
            await page.addScriptTag({ content: windowState });
            const checks = await page.evaluate(async (css) => {
                const style = document.querySelector<HTMLStyleElement>(
                    "#adwcode-performance-style",
                )!;
                style.textContent = css;
                const workbench =
                    document.querySelector<HTMLElement>(".monaco-workbench")!;
                const titlebar =
                    document.querySelector<HTMLElement>(".part.titlebar")!;
                const tab = document.querySelector<HTMLElement>(
                    ".tabs-container > .tab.active",
                )!;
                const clone = tab.cloneNode(true) as HTMLElement;
                tab.parentElement!.append(clone);
                const tabColor =
                    getComputedStyle(clone).color ===
                    getComputedStyle(tab).color;
                clone.remove();
                const sidebar = document.createElement("div");
                sidebar.className = "part sidebar";
                sidebar.innerHTML =
                    '<div class="title"><div class="action-item"><span class="action-label">测试</span></div></div>';
                workbench.append(sidebar);
                await Promise.resolve();
                await Promise.resolve();
                const lateNavigation =
                    getComputedStyle(sidebar.querySelector(".action-label")!)
                        .opacity === "0.65";
                sidebar.remove();
                const label = document.querySelector<HTMLElement>(
                    ".part.editor .editor-actions .action-item:not(.disabled) .action-label",
                )!;
                let highContrast = true;
                for (const mode of ["hc-black", "hc-light"]) {
                    workbench.classList.add(mode);
                    titlebar.classList.remove("inactive");
                    await Promise.resolve();
                    const active = getComputedStyle(label).opacity;
                    titlebar.classList.add("inactive");
                    await Promise.resolve();
                    highContrast &&= active === getComputedStyle(label).opacity;
                    workbench.classList.remove(mode);
                }
                titlebar.classList.remove("inactive");
                await Promise.resolve();
                return {
                    tabColor,
                    lateNavigation,
                    highContrast,
                    stateRestored: !document.querySelector(
                        ".adwcode-window-inactive",
                    ),
                };
            }, css);
            for (const [name, passed] of Object.entries(checks))
                assert.ok(passed, `窗口状态检查失败：${name}`);
            return {
                ...checks,
                comparedElements: captures.full.active.length,
                comparedWithReference: Boolean(captures.reference),
                bodyUnchanged: true,
            };
        };
        metadata.appearanceChecks = [];
        // 在测试页的临时空白子框架中验证早期加载与释放，不连接其他窗口。
        await page.evaluate(() => {
            const iframe = document.createElement("iframe");
            iframe.name = "adwcode-performance-lifecycle";
            iframe.id = "adwcode-performance-lifecycle";
            iframe.hidden = true;
            document.body.append(iframe);
        });
        const frame = page.frame({ name: "adwcode-performance-lifecycle" });
        assert.ok(frame);
        await frame.addScriptTag({ content: windowState });
        const lifecycle = await frame.evaluate(async () => {
            document.body.innerHTML =
                '<div class="monaco-workbench"><div class="part titlebar inactive"></div><div class="part editor"><div class="tabs-container"></div></div></div>';
            await Promise.resolve();
            const tabs =
                document.querySelector<HTMLElement>(".tabs-container")!;
            const lateStartup = tabs.classList.contains(
                "adwcode-window-inactive",
            );
            const 宿主 = window as unknown as Record<
                symbol,
                (() => void) | undefined
            >;
            宿主[Symbol.for("adwcode.windowState")]!();
            document
                .querySelector<HTMLElement>(".titlebar")!
                .classList.remove("inactive");
            document
                .querySelector<HTMLElement>(".titlebar")!
                .classList.add("inactive");
            await Promise.resolve();
            return {
                lateStartup,
                disposed:
                    !tabs.classList.contains("adwcode-window-inactive") &&
                    !宿主[Symbol.for("adwcode.windowState")],
            };
        });
        assert.ok(
            lifecycle.lateStartup && lifecycle.disposed,
            "启动等待及观察器释放必须有效",
        );
        metadata.lifecycleChecks = lifecycle;

        await page
            .locator("#adwcode-performance-lifecycle")
            .evaluate((element) => element.remove());
        for (let mode = 0; mode < themes.length; mode++) {
            if (mode === 1) {
                // 仅更新 Rust 工作台基准 创建的临时用户设置；由 VS Code 原生主题服务响应。
                const settingsPath = path.join(
                    state.profile,
                    "User/settings.json",
                );
                const settings = JSON.parse(
                    fs.readFileSync(settingsPath, "utf8"),
                );
                settings["workbench.colorTheme"] = themes[mode].name;
                fs.writeFileSync(settingsPath, JSON.stringify(settings));
                await page.waitForFunction(() =>
                    document
                        .querySelector<HTMLElement>(".monaco-workbench")!
                        .classList.contains("vs-dark"),
                );
            }
            metadata.appearanceChecks.push({
                mode: themes[mode].type,
                ...(await 验证外观()),
            });
            for (const scenario of selectorsOnly
                ? ["工作台非活动状态切换"]
                : state.scenario === "scroll"
                  ? ["编辑器滚动"]
                  : ["工作台非活动状态切换", "编辑器滚动", "1000 行侧栏压力"]) {
                await page.evaluate((stress) => {
                    document
                        .querySelector("#adwcode-performance-stress")
                        ?.remove();
                    if (stress) {
                        const overlay = document.createElement("div");
                        overlay.id = "adwcode-performance-stress";
                        overlay.className = "part sidebar";
                        overlay.style.cssText =
                            "position:absolute;left:50px;top:50px;width:260px;height:500px;overflow:auto;z-index:100";
                        const list = document.createElement("div");
                        list.className = "explorer-folders-view";
                        for (let i = 0; i < 1000; i++) {
                            const row = document.createElement("div");
                            row.className = "monaco-list-row";
                            row.style.cssText = "position:relative;height:22px";
                            row.innerHTML = `<span class="monaco-icon-label">文件 ${i}</span>`;
                            list.append(row);
                        }
                        overlay.append(list);
                        document
                            .querySelector<HTMLElement>(".monaco-workbench")!
                            .append(overlay);
                    }
                }, scenario === "1000 行侧栏压力");
                // 首轮预热，随后交替测试原生 CSS 与附加 CSS，避免固定顺序偏差。
                for (let round = -1; round < runs; round++) {
                    for (const variant of round % 2 === 0
                        ? variants
                        : [...variants].reverse()) {
                        const enabled = variant.enabled;
                        await page.evaluate((variant) => {
                            const style =
                                document.querySelector<HTMLStyleElement>(
                                    "#adwcode-performance-style",
                                )!;
                            if (style.textContent !== variant.css)
                                style.textContent = variant.css;
                            style.sheet!.disabled = !variant.enabled;
                            document
                                .querySelector<HTMLElement>(".part.titlebar")!
                                .classList.remove("inactive");
                            document.body.offsetHeight;
                        }, variant);
                        if (scenario === "编辑器滚动") {
                            // 样式或主题切换后重新定位可见编辑器，避免沿用早期窗口坐标。
                            const box = await page
                                .locator(".monaco-editor:visible")
                                .first()
                                .boundingBox();
                            assert.ok(box && box.height > 120);
                            const position = {
                                x: box.x + box.width / 2,
                                y: box.y + box.height / 2,
                            };
                            assert.ok(
                                await page.evaluate(
                                    (position) =>
                                        Boolean(
                                            document
                                                .elementFromPoint(
                                                    position.x,
                                                    position.y,
                                                )
                                                ?.closest(".monaco-editor"),
                                        ),
                                    position,
                                ),
                                "滚轮目标必须位于可见编辑器",
                            );
                            await page.mouse.click(position.x, position.y);
                            await page.keyboard.press("Control+Home");
                            await page.mouse.move(position.x, position.y);
                            await page.waitForTimeout(100);
                        }
                        await page.evaluate(
                            () =>
                                new Promise<void>((resolve) =>
                                    requestAnimationFrame(() =>
                                        requestAnimationFrame(() => resolve()),
                                    ),
                                ),
                        );
                        const before = await 读取指标();
                        let sample: 采样结果;
                        if (scenario === "编辑器滚动") {
                            const start = 节点计时.now();
                            const frames = [];
                            const visible = new Set();
                            let last;
                            for (let i = 0; i < 50; i++) {
                                // 使用 CDP 的真实输入；合成 DOM WheelEvent 在此构建中不会滚动。
                                await page.mouse.wheel(0, 120);
                                const state = await page.evaluate(
                                    () =>
                                        new Promise<{
                                            timestamp: number;
                                            line: string | null | undefined;
                                        }>((resolve) =>
                                            requestAnimationFrame((timestamp) =>
                                                resolve({
                                                    timestamp,
                                                    line: document.querySelector(
                                                        ".monaco-editor .view-line",
                                                    )?.textContent,
                                                }),
                                            ),
                                        ),
                                );
                                if (last !== undefined)
                                    frames.push(state.timestamp - last);
                                last = state.timestamp;
                                visible.add(state.line);
                            }
                            sample = {
                                wall_ms: 节点计时.now() - start,
                                iterations: 50,
                                frames,
                                visibleLineStates: visible.size,
                                nodes: await page.locator("*").count(),
                            };
                        } else {
                            sample = await page.evaluate(
                                async ({ scenario, iterations }) => {
                                    const titlebar =
                                        document.querySelector<HTMLElement>(
                                            ".part.titlebar",
                                        )!;
                                    const workbench =
                                        document.querySelector<HTMLElement>(
                                            ".monaco-workbench",
                                        )!;
                                    const stress = document.querySelector(
                                        "#adwcode-performance-stress",
                                    );
                                    const start = window.performance.now();
                                    for (let i = 0; i < iterations; i++) {
                                        titlebar.classList.toggle(
                                            "inactive",
                                            i % 2 === 0,
                                        );
                                        await Promise.resolve();
                                        getComputedStyle(
                                            stress?.lastElementChild
                                                ?.lastElementChild || workbench,
                                        ).color;
                                        workbench.offsetHeight;
                                    }
                                    titlebar.classList.remove("inactive");
                                    await Promise.resolve();
                                    return {
                                        wall_ms: window.performance.now() - start,
                                        iterations,
                                        frames: [],
                                        nodes: document.querySelectorAll("*")
                                            .length,
                                    };
                                },
                                { scenario, iterations: 200 },
                            );
                        }
                        const after = await 读取指标();
                        const delta: Record<string, number> = {};
                        for (const key of [
                            "TaskDuration",
                            "ScriptDuration",
                            "RecalcStyleDuration",
                            "LayoutDuration",
                            "RecalcStyleCount",
                            "LayoutCount",
                        ])
                            delta[key] = after[key] - before[key];
                        if (scenario === "编辑器滚动")
                            assert.ok(
                                (sample.visibleLineStates ?? 0) > 1,
                                "滚动场景必须实际改变可见内容",
                            );
                        if (round >= 0)
                            results.push({
                                mode: themes[mode].type,
                                scenario,
                                variant: variant.name,
                                enabled,
                                round,
                                ...sample,
                                metrics: delta,
                            });
                    }
                }
                保存(false);
                console.log(`${themes[mode].name}：${scenario}完成`);
            }
        }
        保存(true);
        console.log(`原始结果：${output}`);
    } finally {
        await browser.close();
    }
}
运行基准().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
