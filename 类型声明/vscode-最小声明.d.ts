// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//
// VS Code 扩展 API 的最小类型面：只声明 扩展/扩展.js 用到的成员。
// 官方 @types/vscode 不在本仓库依赖中；本文件由 `meson compile -C builddir 类型检查` 的 tsc 与
// 编辑器共同消费，真实签名见 VS Code 安装目录的
// resources/app/out/vscode-dts/vscode.d.ts。

/** 与官方 typings 一致：`Thenable` 是全局类型（供 `await` 与 `Promise.all` 使用）。 */
interface Thenable<T> extends PromiseLike<T> {}

declare module "vscode" {
    /** 可释放资源；`context.subscriptions` 中存放的就是它。 */
    export interface Disposable {
        dispose(): unknown;
    }

    export interface Uri {
        readonly scheme: string;
        readonly fsPath: string;
        toString(): string;
    }

    export namespace Uri {
        function parse(value: string): Uri;
        function file(path: string): Uri;
    }

    export enum ConfigurationTarget {
        Global = 1,
        Workspace = 2,
        WorkspaceFolder = 3,
    }

    export interface WorkspaceConfiguration {
        get<T>(section: string): T | undefined;
        get<T>(section: string, defaultValue: T): T;
        update(
            section: string,
            value: unknown,
            configurationTarget?: ConfigurationTarget | boolean | null
        ): Thenable<void>;
        inspect<T>(section: string): { defaultValue?: T; globalValue?: T; workspaceValue?: T; workspaceFolderValue?: T } | undefined;
    }

    export interface Memento {
        get<T>(key: string): T | undefined;
        update(key: string, value: unknown): Thenable<void>;
    }

    export interface ExtensionContext {
        readonly extensionPath: string;
        readonly subscriptions: Disposable[];
        readonly globalState: Memento;
    }

    export interface Clipboard {
        writeText(value: string): Thenable<void>;
    }

    export namespace workspace {
        function getConfiguration(section?: string): WorkspaceConfiguration;
    }

    export interface OutputChannel {
        replace(value: string): void;
        show(preserveFocus?: boolean): void;
        dispose(): unknown;
    }

    export namespace window {
        function createOutputChannel(name: string): OutputChannel;
        function showInformationMessage(
            message: string,
            ...items: string[]
        ): Thenable<string | undefined>;
        function showInformationMessage(
            message: string,
            options: { modal?: boolean },
            ...items: string[]
        ): Thenable<string | undefined>;
        function showWarningMessage(
            message: string,
            ...items: string[]
        ): Thenable<string | undefined>;
        function showErrorMessage(
            message: string,
            ...items: string[]
        ): Thenable<string | undefined>;
    }

    export namespace commands {
        function registerCommand(
            command: string,
            callback: (...args: any[]) => any,
            thisArg?: any
        ): Disposable;
        function executeCommand<T>(command: string, ...rest: any[]): Thenable<T | undefined>;
    }

    export namespace extensions {
        function getExtension<T>(extensionId: string): { readonly packageJSON: any } | undefined;
    }

    export namespace env {
        const appRoot: string;
        const clipboard: Clipboard;
    }
}
