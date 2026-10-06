// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! `adwcode` 工具入口：只负责参数收集与退出码。

use std::process::ExitCode;

fn main() -> ExitCode {
    let 参数: Vec<String> = std::env::args().skip(1).collect();
    match adwcode::命令::执行(&参数) {
        Ok(()) => ExitCode::SUCCESS,
        Err(错误) => {
            eprintln!("错误：{错误}");
            ExitCode::FAILURE
        }
    }
}
