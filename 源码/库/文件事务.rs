// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 同目录暂存、逐文件原子替换及错误回滚。进程被强制终止时不保证批次原子性。
use crate::错误::{工具错误, 结果};
use std::io::Write;
use std::path::{Path, PathBuf};

/// 拒绝文件及任一祖先的符号链接，避免读取或替换仓库外数据。
pub fn 校验普通路径(路径: &Path) -> 结果<()> {
    for 项 in std::path::absolute(路径)?.ancestors() {
        match std::fs::symlink_metadata(项) {
            Ok(元数据) if 元数据.file_type().is_symlink() => {
                return Err(工具错误::新(format!("拒绝符号链接：{}", 项.display())));
            }
            Ok(_) => (),
            Err(错误) if 错误.kind() == std::io::ErrorKind::NotFound => (),
            Err(错误) => return Err(错误.into()),
        }
    }
    Ok(())
}

fn 暂存(路径: &Path, 内容: &[u8]) -> 结果<tempfile::NamedTempFile> {
    let 父目录 = 路径
        .parent()
        .ok_or_else(|| 工具错误::新("文件没有父目录"))?;
    std::fs::create_dir_all(父目录)?;
    let mut 文件 = tempfile::NamedTempFile::new_in(父目录)?;
    if let Ok(元数据) = std::fs::metadata(路径) {
        文件.as_file().set_permissions(元数据.permissions())?;
    }
    文件.write_all(内容)?;
    文件.as_file().sync_all()?;
    Ok(文件)
}

/// `None` 删除文件；所有输入与暂存完成后才替换，失败时恢复已改动项。
pub fn 写入批次(操作: &[(PathBuf, Option<Vec<u8>>)]) -> 结果<()> {
    执行批次(操作, &|_| Ok(()))
}

fn 执行批次(
    操作: &[(PathBuf, Option<Vec<u8>>)],
    替换前: &dyn Fn(usize) -> 结果<()>,
) -> 结果<()> {
    let mut 准备 = Vec::new();
    let mut 已见 = std::collections::HashSet::new();
    for (路径, 内容) in 操作 {
        let 路径 = std::path::absolute(路径)?;
        if !已见.insert(路径.clone()) {
            return Err(工具错误::新("批次包含重复路径"));
        }
        校验普通路径(&路径)?;
        let 原内容 = match std::fs::read(&路径) {
            Ok(内容) => Some(内容),
            Err(错误) if 错误.kind() == std::io::ErrorKind::NotFound => None,
            Err(错误) => return Err(错误.into()),
        };
        let 暂存文件 = 内容.as_ref().map(|内容| 暂存(&路径, 内容)).transpose()?;
        let 备份 = 原内容.as_ref().map(|内容| 暂存(&路径, 内容)).transpose()?;
        准备.push((路径, 暂存文件, 备份));
    }
    for 序号 in 0..准备.len() {
        let 本次 = (|| -> 结果<()> {
            替换前(序号)?;
            let (路径, 暂存文件, _) = &mut 准备[序号];
            if let Some(文件) = 暂存文件.take() {
                文件
                    .persist(路径)
                    .map_err(|错误| 工具错误::新(format!("原子替换失败：{错误}")))?;
            } else if 路径.exists() {
                std::fs::remove_file(路径)?;
            }
            Ok(())
        })();
        if let Err(错误) = 本次 {
            let mut 恢复失败 = Vec::new();
            for (路径, _, 备份) in 准备[..序号].iter_mut().rev() {
                let 恢复 = if let Some(文件) = 备份.take() {
                    文件
                        .persist(&*路径)
                        .map(|_| ())
                        .map_err(|错误| 错误.to_string())
                } else {
                    std::fs::remove_file(&*路径).map_err(|错误| 错误.to_string())
                };
                if let Err(失败) = 恢复 {
                    恢复失败.push(失败);
                }
            }
            return Err(工具错误::新(format!(
                "{错误}{}",
                if 恢复失败.is_empty() {
                    "；已恢复原文件".to_string()
                } else {
                    format!("；恢复未完成：{}", 恢复失败.join("；"))
                }
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;
    #[test]
    fn 替换失败恢复旧文件和删除新增文件() {
        let 根 = tempfile::tempdir().unwrap();
        let 旧 = 根.path().join("旧");
        let 新 = 根.path().join("新");
        std::fs::write(&旧, "原值".as_bytes()).unwrap();
        let 操作 = vec![
            (旧.clone(), Some("修改".as_bytes().to_vec())),
            (新.clone(), Some("新增".as_bytes().to_vec())),
            (根.path().join("故障"), None),
        ];
        assert!(
            执行批次(&操作, &|序号| if 序号 == 2 {
                Err(工具错误::新("注入替换故障"))
            } else {
                Ok(())
            })
            .is_err()
        );
        assert_eq!(std::fs::read(&旧).unwrap(), "原值".as_bytes());
        assert!(!新.exists());
        assert_eq!(std::fs::read_dir(根.path()).unwrap().count(), 1);
    }
    #[test]
    fn 预检失败不修改先前文件() {
        let 根 = tempfile::tempdir().unwrap();
        let 路径 = 根.path().join("原文件");
        std::fs::write(&路径, "原值".as_bytes()).unwrap();
        assert!(
            写入批次(&[
                (路径.clone(), Some(vec![])),
                (根.path().to_path_buf(), Some(vec![]))
            ])
            .is_err()
        );
        assert_eq!(std::fs::read(路径).unwrap(), "原值".as_bytes());
    }
    #[cfg(unix)]
    #[test]
    fn 符号链接及祖先被拒绝() {
        let 根 = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(根.path(), 根.path().join("链接")).unwrap();
        assert!(校验普通路径(&根.path().join("链接/文件")).is_err());
    }
}
