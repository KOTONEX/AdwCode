// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 仓库根目录定位：优先环境变量，其次从当前目录与可执行文件位置向上查找。

use std::path::{Path, PathBuf};

use crate::错误::{工具错误, 结果};

/// 根目录必须同时包含 `package.json` 与 `主题/`。
#[must_use]
pub fn 向上查找(起始: &Path) -> Option<PathBuf> {
    let mut 当前 = Some(起始);
    while let Some(目录) = 当前 {
        if 目录.join("package.json").is_file() && 目录.join("主题").is_dir() {
            return Some(目录.to_path_buf());
        }
        当前 = 目录.parent();
    }
    None
}

/// 定位仓库根目录；`ADWCODE根目录` 可显式指定。
pub fn 根目录() -> 结果<PathBuf> {
    if let Ok(值) = std::env::var("ADWCODE根目录") {
        let 路径 = PathBuf::from(值);
        if 路径.is_dir() {
            return Ok(路径);
        }
        return Err(工具错误::新(format!(
            "ADWCODE根目录 不是有效目录：{}",
            路径.display()
        )));
    }
    if let Ok(当前) = std::env::current_dir()
        && let Some(根) = 向上查找(&当前)
    {
        return Ok(根);
    }
    if let Ok(程序) = std::env::current_exe()
        && let Some(目录) = 程序.parent()
        && let Some(根) = 向上查找(目录)
    {
        return Ok(根);
    }
    Err(工具错误::新(
        "未找到 AdwCode 仓库根目录：请在仓库内运行，或用 ADWCODE根目录 指定",
    ))
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 从仓库根目录可以定位() {
        let 根 = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(向上查找(根), Some(根.to_path_buf()));
    }
}
