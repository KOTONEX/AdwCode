// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 严格编译 TypeScript，在临时目录生成后事务写入运行产物。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::错误::{工具错误, 结果};

pub const 产物目录: &str = "builddir/脚本";

fn 收集(目录: &Path, 文件: &mut Vec<PathBuf>) -> 结果<()> {
    crate::文件事务::校验普通路径(目录)?;
    if !目录.exists() {
        return Ok(());
    }
    for 条目 in std::fs::read_dir(目录)? {
        let 路径 = 条目?.path();
        crate::文件事务::校验普通路径(&路径)?;
        if 路径.is_dir() {
            收集(&路径, 文件)?;
        } else if 路径.is_file() {
            文件.push(路径);
        }
    }
    文件.sort();
    Ok(())
}

pub fn 源文件(根目录: &Path) -> 结果<Vec<PathBuf>> {
    let mut 文件 = Vec::new();
    for 目录 in ["扩展", "附加外观", "测试", "基准", "类型声明"] {
        收集(&根目录.join(目录), &mut 文件)?;
    }
    文件.retain(|路径| 路径.extension().is_some_and(|后缀| 后缀 == "ts"));
    Ok(文件)
}

pub fn 产物文件(根目录: &Path) -> 结果<Vec<PathBuf>> {
    Ok(源文件(根目录)?
        .into_iter()
        .filter(|路径| !路径.to_string_lossy().ends_with(".d.ts"))
        .map(|路径| {
            根目录
                .join(产物目录)
                .join(路径.strip_prefix(根目录).unwrap())
                .with_extension("js")
        })
        .collect())
}

fn 成功输出(根目录: &Path, 命令: &mut Command) -> 结果<String> {
    let 输出 = 命令.current_dir(根目录).output().map_err(|错误| {
        工具错误::带来源(
            "无法运行 tsc，请安装最新稳定版 TypeScript".to_string(),
            错误,
        )
    })?;
    let 文本 = format!(
        "{}{}",
        String::from_utf8_lossy(&输出.stdout),
        String::from_utf8_lossy(&输出.stderr)
    );
    if !输出.status.success() {
        return Err(工具错误::新(format!(
            "TypeScript 编译失败：\n{}",
            文本.trim_end()
        )));
    }
    Ok(文本)
}

fn 校验稳定编译器(文本: &str) -> 结果<()> {
    let 版本 = 文本
        .trim()
        .strip_prefix("Version ")
        .and_then(|版本| semver::Version::parse(版本).ok())
        .filter(|版本| 版本.pre.is_empty());
    if 版本.is_none() {
        return Err(工具错误::新(format!(
            "需要最新稳定版 TypeScript，请更新编译器；当前输出：{}",
            文本.trim()
        )));
    }
    Ok(())
}

/// 只有全部文件编译成功，才更新产物并删除过时文件；失败不使用旧产物。
pub fn 编译(根目录: &Path) -> 结果<Vec<PathBuf>> {
    let 文件 = 产物文件(根目录)?;
    if 文件.is_empty() {
        return Ok(文件);
    }
    crate::文件事务::校验普通路径(&根目录.join("tsconfig.json"))?;
    let mut 旧文件 = Vec::new();
    收集(&根目录.join(产物目录), &mut 旧文件)?;
    let 版本 = 成功输出(根目录, Command::new("tsc").arg("--version"))?;
    校验稳定编译器(&版本)?;
    let 临时 = tempfile::Builder::new().prefix("adwcode-ts-").tempdir()?;
    成功输出(
        根目录,
        Command::new("tsc")
            .args([
                "--project",
                "tsconfig.json",
                "--pretty",
                "false",
                "--incremental",
                "false",
            ])
            .arg("--outDir")
            .arg(临时.path()),
    )?;
    let mut 实际文件 = Vec::new();
    收集(临时.path(), &mut 实际文件)?;
    for 路径 in &实际文件 {
        let 相对 = 路径.strip_prefix(临时.path()).unwrap();
        if !文件.contains(&根目录.join(产物目录).join(相对)) {
            return Err(工具错误::新(format!(
                "编译器生成了未登记的产物：{}；请将源码放入脚本目录并同步构建清单",
                相对.display()
            )));
        }
    }
    let mut 操作 = Vec::new();
    for 路径 in &文件 {
        let 相对 = 路径.strip_prefix(根目录.join(产物目录)).unwrap();
        let 来源 = 临时.path().join(相对);
        crate::文件事务::校验普通路径(&来源)?;
        操作.push((
            路径.clone(),
            Some(std::fs::read(&来源).map_err(|错误| {
                工具错误::带来源(format!("编译器未生成预期脚本：{}", 相对.display()), 错误)
            })?),
        ));
    }
    for 路径 in 旧文件 {
        if !文件.contains(&路径) {
            操作.push((路径, None));
        }
    }
    crate::文件事务::写入批次(&操作)?;
    Ok(文件)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 稳定编译器不固定数字版本且拒绝预发布() {
        for 文本 in ["Version 7.0.2\n", "Version 99.0.0\n"] {
            校验稳定编译器(文本).unwrap();
        }
        for 文本 in [
            "Version 7.1.0-dev.20261008",
            "Version 7.1.0-beta",
            "Version 7.1.0-rc",
            "Version nightly",
            "",
        ] {
            assert!(校验稳定编译器(文本).is_err());
        }
    }

    #[test]
    fn 编译成功替换产物但类型错误保留旧内容() {
        let 临时 = tempfile::tempdir().unwrap();
        let 根 = 临时.path();
        std::fs::create_dir(根.join("扩展")).unwrap();
        std::fs::write(根.join("tsconfig.json"), r#"{"compilerOptions":{"target":"ES2022","strict":true,"noEmitOnError":true,"types":[],"rootDir":"."},"include":["扩展/*.ts"]}"#).unwrap();
        std::fs::write(根.join("扩展/扩展.ts"), "const 名称: string = '正确';\n").unwrap();
        let 文件 = 编译(根).unwrap();
        let 旧内容 = std::fs::read(&文件[0]).unwrap();
        std::fs::write(根.join("builddir/脚本/过时.js"), "过时").unwrap();
        std::fs::write(根.join("扩展/扩展.ts"), "const 名称: string = 123;\n").unwrap();
        assert!(编译(根).unwrap_err().消息.contains("TypeScript 编译失败"));
        assert_eq!(std::fs::read(&文件[0]).unwrap(), 旧内容);
        assert!(根.join("builddir/脚本/过时.js").exists());
        std::fs::write(根.join("遗漏.ts"), "export const 名称 = '遗漏';\n").unwrap();
        std::fs::write(
            根.join("扩展/扩展.ts"),
            "import { 名称 } from '../遗漏';\nconsole.log(名称);\n",
        )
        .unwrap();
        assert!(编译(根).unwrap_err().消息.contains("未登记的产物：遗漏.js"));
        assert_eq!(std::fs::read(&文件[0]).unwrap(), 旧内容);
        assert!(根.join("builddir/脚本/过时.js").exists());
        std::fs::write(根.join("扩展/扩展.ts"), "const 名称: string = '更新';\n").unwrap();
        编译(根).unwrap();
        assert!(!根.join("builddir/脚本/过时.js").exists());
        assert_ne!(std::fs::read(&文件[0]).unwrap(), 旧内容);
        #[cfg(unix)]
        {
            let 外部 = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(外部.path(), 根.join("builddir/脚本/外部")).unwrap();
            assert!(编译(根).is_err());
            assert!(std::fs::read_dir(外部.path()).unwrap().next().is_none());
        }
    }
}
