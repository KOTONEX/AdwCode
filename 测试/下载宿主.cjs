// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 外部测试工具安装目录由 CI 提供，不成为扩展的运行时依赖。
const path = require('node:path');
require(path.resolve(process.argv[2], 'node_modules/@vscode/test-electron'))
  .downloadAndUnzipVSCode('stable').then(文件 => console.log(文件), 错误 => { console.error(错误); process.exitCode = 1; });
