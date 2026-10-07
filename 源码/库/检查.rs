// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 静态检查、类型检查与离线测试的聚合入口，取代 Meson 目标。
//!
//! - `检查`：cargo fmt/clippy/test、主题与图标校验、JavaScript 语法检查、
//!   清单默认设置验证、TypeScript 类型检查与离线 JS 测试。
//! - `格式化`：`cargo fmt`。
//! - `类型检查`：`cargo check --all-targets` 与 `tsc --noEmit`。

use std::path::Path;
use std::process::Command;

use crate::错误::{工具错误, 结果};

/// 运行外部程序并返回合并后的输出；失败时返回包含输出的错误。
fn 运行(根目录: &Path, 程序: &str, 参数: &[&str]) -> 结果<String> {
    let 输出 = Command::new(程序)
        .args(参数)
        .current_dir(根目录)
        .output()
        .map_err(|错误| {
            工具错误::带来源(format!("无法运行 {程序}，请确认已安装：{错误}"), 错误)
        })?;
    let 文本 = format!(
        "{}{}",
        String::from_utf8_lossy(&输出.stdout),
        String::from_utf8_lossy(&输出.stderr)
    );
    if !输出.status.success() {
        return Err(工具错误::新(format!(
            "{程序} {} 失败：\n{}",
            参数.join(" "),
            文本.trim_end()
        )));
    }
    Ok(文本)
}

fn 要求程序(根目录: &Path, 程序: &str) -> 结果<()> {
    运行(根目录, 程序, &["--version"]).map(|_| ())
}

/// 收集需要语法检查的 JavaScript 文件。
pub fn 脚本文件(根目录: &Path) -> 结果<Vec<String>> {
    let mut 文件 = vec![
        "测试/验证状态.cjs".to_string(),
        "基准/扩展基准.cjs".to_string(),
        "基准/工作台基准.cjs".to_string(),
    ];
    收集模块(&根目录.join("扩展"), 根目录, &mut 文件)?;
    收集模块(&根目录.join("测试"), 根目录, &mut 文件)?;
    let 附加目录 = 根目录.join("附加外观");
    let mut 附加: Vec<String> = std::fs::read_dir(&附加目录)?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|条目| {
            条目
                .path()
                .extension()
                .is_some_and(|扩展| 扩展 == "js" || 扩展 == "cjs")
        })
        .map(|条目| format!("附加外观/{}", 条目.file_name().to_string_lossy()))
        .collect();
    附加.sort();
    文件.extend(附加);
    Ok(文件)
}

fn 收集模块(目录: &Path, 根目录: &Path, 文件: &mut Vec<String>) -> 结果<()> {
    for 条目 in std::fs::read_dir(目录)? {
        let 路径 = 条目?.path();
        crate::文件事务::校验普通路径(&路径)?;
        if 路径.is_dir() {
            收集模块(&路径, 根目录, 文件)?;
        } else if 路径
            .extension()
            .is_some_and(|扩展| 扩展 == "js" || 扩展 == "cjs")
        {
            文件.push(
                路径
                    .strip_prefix(根目录)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    文件.sort();
    文件.dedup();
    Ok(())
}

/// `adwcode 检查`：完整离线检查。
pub fn 检查(根目录: &Path) -> 结果<()> {
    println!("cargo fmt --check");
    运行(根目录, "cargo", &["fmt", "--check"])?;
    println!("cargo clippy");
    运行(
        根目录,
        "cargo",
        &["clippy", "--all-targets", "--quiet", "--", "-D", "warnings"],
    )?;
    println!("cargo test");
    运行(根目录, "cargo", &["test", "--quiet"])?;
    println!("验证设置");
    crate::验证设置::验证设置(根目录)?;
    println!("校验");
    let 失败 = crate::主题生成::校验(根目录)?;
    if 失败 != 0 {
        return Err(工具错误::新(format!("校验失败：{失败} 项")));
    }
    要求程序(根目录, "node")?;
    for 文件 in 脚本文件(根目录)? {
        println!("node --check {文件}");
        运行(根目录, "node", &["--check", &文件])?;
    }
    println!("tsc --noEmit");
    运行(根目录, "tsc", &["--noEmit"])?;
    println!("离线 JS 测试");
    运行(根目录, "node", &["测试/验证状态.cjs"])?;
    println!("全部检查通过");
    Ok(())
}

/// `adwcode 格式化`：格式化全部 Rust 源码。
pub fn 格式化(根目录: &Path) -> 结果<()> {
    运行(根目录, "cargo", &["fmt"])?;
    Ok(())
}

/// `adwcode 类型检查`：Rust 编译检查与 TypeScript 类型检查。
pub fn 类型检查(根目录: &Path) -> 结果<()> {
    println!("cargo check --all-targets");
    运行(根目录, "cargo", &["check", "--all-targets", "--quiet"])?;
    要求程序(根目录, "tsc")?;
    println!("tsc --noEmit");
    运行(根目录, "tsc", &["--noEmit"])?;
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 脚本清单包含必需文件() {
        let 根 = Path::new(env!("CARGO_MANIFEST_DIR"));
        let 文件 = 脚本文件(根).expect("收集脚本");
        for 必需 in [
            "扩展/扩展.js",
            "附加外观/窗口状态.js",
            "测试/验证状态.cjs",
            "基准/扩展基准.cjs",
        ] {
            assert!(文件.contains(&必需.to_string()), "缺少 {必需}");
        }
    }
}
