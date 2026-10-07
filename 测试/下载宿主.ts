// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 外部测试工具不成为扩展运行时依赖；固定版本与缓存路径由 CI 提供。
import * as path from "path";
(
    require(
        path.resolve(process.argv[2], "node_modules/@vscode/test-electron"),
    ) as {
        downloadAndUnzipVSCode(options: {
            version: string;
            cachePath: string;
        }): Promise<string>;
    }
)
    .downloadAndUnzipVSCode({
        version: process.argv[3] || "stable",
        cachePath: process.argv[4] || path.resolve(".vscode-test"),
    })
    .then(
        (文件) => console.log(文件),
        (错误) => {
            console.error(错误);
            process.exitCode = 1;
        },
    );
