// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//
// Node 运行时与环境的最小类型面：只声明 扩展/扩展.js 用到的 API。
// 官方 @types/node 不在本仓库依赖中。

declare function require(id: string): unknown;
declare var module: { exports: unknown };
declare var process: {
    readonly platform: string;
    readonly env: Record<string, string | undefined>;
    readonly argv: string[];
    readonly version: string;
    exitCode?: number;
};
declare function setTimeout(
    callback: (...args: unknown[]) => void,
    ms?: number,
): unknown;
declare function clearTimeout(timeout: unknown): void;

declare module "stream" {
    export interface Readable {
        on(event: "data", listener: (chunk: unknown) => void): this;
        on(event: string, listener: (...args: unknown[]) => void): this;
    }
}

declare module "child_process" {
    import { Readable } from "stream";

    export interface ChildProcess {
        readonly stdout: Readable | null;
        readonly stderr: Readable | null;
        on(event: "error", listener: (error: Error) => void): this;
        on(
            event: "exit",
            listener: (code: number | null, signal: string | null) => void,
        ): this;
        kill(signal?: string): boolean;
    }

    export function execFile(
        file: string,
        args?: readonly string[] | null,
        options?: { timeout?: number },
    ): ChildProcess;

    export function execFile(
        file: string,
        args: readonly string[] | null | undefined,
        options: { timeout?: number },
        callback: (error: Error | null, stdout: string, stderr: string) => void,
    ): ChildProcess;
}

declare module "fs" {
    export function existsSync(path: string): boolean;
    export function readFileSync(path: string, encoding: "utf8"): string;
    export const promises: {
        mkdir(
            path: string,
            options?: { recursive?: boolean; mode?: number },
        ): Promise<string | undefined>;
        rename(source: string, target: string): Promise<void>;
        unlink(path: string): Promise<void>;
        copyFile(source: string, target: string): Promise<void>;
        writeFile(path: string, data: string, encoding: "utf8"): Promise<void>;
    };
}

declare module "os" {
    export function homedir(): string;
}

declare module "path" {
    export function join(...parts: string[]): string;
    export function resolve(...parts: string[]): string;
    export function basename(path: string): string;
    export function isAbsolute(path: string): boolean;
}

// 测试与基准使用的 Node API；动态 require 的结果必须在调用边界校验或明确收窄。
declare const __dirname: string;
declare const global: { gc?: () => void };
declare module "assert/strict" {
    export function ok(value: unknown, message?: string): asserts value;
    export function equal(
        actual: unknown,
        expected: unknown,
        message?: string,
    ): void;
    export function notEqual(
        actual: unknown,
        expected: unknown,
        message?: string,
    ): void;
    export function deepEqual(
        actual: unknown,
        expected: unknown,
        message?: string,
    ): void;
    export function notDeepEqual(
        actual: unknown,
        expected: unknown,
        message?: string,
    ): void;
    export function throws(block: () => unknown, expected?: RegExp): void;
    export function rejects(
        block: Promise<unknown>,
        expected?: RegExp,
    ): Promise<void>;
}
declare module "fs" {
    export function writeFileSync(path: string, data: string): void;
    export function mkdtempSync(prefix: string): string;
    export function mkdirSync(
        path: string,
        options?: { recursive?: boolean },
    ): string | undefined;
    export function rmSync(
        path: string,
        options?: { recursive?: boolean; force?: boolean },
    ): void;
}
declare module "os" {
    export function tmpdir(): string;
}
declare module "path" {
    export function dirname(path: string): string;
}
declare module "perf_hooks" {
    export const performance: { now(): number };
}
declare module "vm" {
    export function runInNewContext(
        code: string,
        context: object,
        options: { filename: string },
    ): unknown;
}
