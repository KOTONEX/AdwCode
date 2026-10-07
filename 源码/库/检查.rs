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

/// 列出 TypeScript 编译后的运行脚本。
pub fn 脚本文件(根目录: &Path) -> 结果<Vec<String>> {
    Ok(crate::脚本构建::产物文件(根目录)?
        .into_iter()
        .map(|路径| {
            路径
                .strip_prefix(根目录)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect())
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
    println!("严格编译全部 TypeScript");
    crate::脚本构建::编译(根目录)?;
    要求程序(根目录, "node")?;
    for 文件 in 脚本文件(根目录)? {
        println!("node --check {文件}");
        运行(根目录, "node", &["--check", &文件])?;
    }
    println!("离线脚本测试");
    运行(根目录, "node", &["builddir/脚本/测试/验证状态.js"])?;
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
            "builddir/脚本/扩展/扩展.js",
            "builddir/脚本/附加外观/窗口状态.js",
            "builddir/脚本/测试/验证状态.js",
            "builddir/脚本/基准/扩展基准.js",
        ] {
            assert!(文件.contains(&必需.to_string()), "缺少 {必需}");
        }
    }
}
