// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 仓库根目录定位：优先环境变量，其次从当前目录与可执行文件位置向上查找。

use std::path::{Path, PathBuf};

use crate::错误::{工具错误, 结果};

fn 是项目根目录(目录: &Path) -> bool {
    目录.join("主题").is_dir()
        && std::fs::read(目录.join("package.json"))
            .ok()
            .and_then(|字节| serde_json::from_slice::<serde_json::Value>(&字节).ok())
            .is_some_and(|清单| {
                清单["name"]
                    .as_str()
                    .is_some_and(|名称| 名称.eq_ignore_ascii_case("adwcode"))
            })
}

/// 根目录必须有 AdwCode 清单与主题目录，不能误用其他扩展仓库。
#[must_use]
pub fn 向上查找(起始: &Path) -> Option<PathBuf> {
    起始
        .ancestors()
        .find(|目录| 是项目根目录(目录))
        .map(Path::to_path_buf)
}

/// 定位仓库根目录；`ADWCODE根目录` 可显式指定。
pub fn 根目录() -> 结果<PathBuf> {
    if let Ok(值) = std::env::var("ADWCODE根目录") {
        let 路径 = PathBuf::from(值);
        if 是项目根目录(&路径) {
            return Ok(路径);
        }
        return Err(工具错误::新(format!(
            "ADWCODE根目录 不是有效的 AdwCode 项目目录：{}",
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
    #[test]
    fn 不接受其他项目或破损清单() {
        let 根 = tempfile::tempdir().unwrap();
        std::fs::create_dir(根.path().join("主题")).unwrap();
        for 内容 in [r#"{"name":"其他项目"}"#, "{"] {
            std::fs::write(根.path().join("package.json"), 内容).unwrap();
            assert_eq!(向上查找(根.path()), None);
        }
        std::fs::write(根.path().join("package.json"), r#"{"name":"AdwCode"}"#).unwrap();
        assert_eq!(
            向上查找(&根.path().join("源码/库")),
            Some(根.path().to_path_buf())
        );
    }
}
