// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// Playwright 由调用者提供；此处只描述工作台基准实际使用的外部 API。
declare module "playwright" {
    interface 求值环境 {
        evaluate<返回>(函数: () => 返回 | Promise<返回>): Promise<返回>;
        evaluate<返回, 参数>(
            函数: (参数: 参数) => 返回 | Promise<返回>,
            参数: 参数,
        ): Promise<返回>;
        addScriptTag(选项: { content: string }): Promise<unknown>;
    }
    interface 定位器 {
        isVisible(): Promise<boolean>;
        click(): Promise<void>;
        first(): 定位器;
        boundingBox(): Promise<{
            x: number;
            y: number;
            width: number;
            height: number;
        } | null>;
        count(): Promise<number>;
        evaluate<返回>(函数: (元素: HTMLElement) => 返回): Promise<返回>;
    }
    interface 调试会话 {
        send(
            方法: "Performance.getMetrics",
        ): Promise<{ metrics: { name: string; value: number }[] }>;
        send(方法: "Performance.enable"): Promise<void>;
    }
    interface 浏览器上下文 {
        pages(): 页面[];
        newCDPSession(页面: 页面): Promise<调试会话>;
    }
    interface 页面 extends 求值环境 {
        url(): string;
        title(): Promise<string>;
        context(): 浏览器上下文;
        waitForSelector(
            选择器: string,
            选项: { timeout: number },
        ): Promise<unknown>;
        waitForTimeout(毫秒: number): Promise<void>;
        waitForFunction(函数: () => boolean): Promise<unknown>;
        locator(选择器: string): 定位器;
        addStyleTag(选项: { content: string }): Promise<unknown>;
        frame(选项: { name: string }): 求值环境 | null;
        mouse: {
            click(x: number, y: number): Promise<void>;
            move(x: number, y: number): Promise<void>;
            wheel(x: number, y: number): Promise<void>;
        };
        keyboard: { press(键: string): Promise<void> };
    }
    interface 浏览器 {
        contexts(): 浏览器上下文[];
        version(): string;
        close(): Promise<void>;
    }
    export const chromium: {
        connectOverCDP(
            地址: string,
            选项: { timeout: number },
        ): Promise<浏览器>;
    };
}
