// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 根据清单版本和 Git 主线归属判断候选与市场发布权限。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::变更日志::执行git;
use crate::错误::{工具错误, 结果};

#[derive(Debug, PartialEq, Eq)]
pub struct 发布判断 {
    pub 版本: String,
    pub 候选: bool,
    pub 市场: bool,
}

impl 发布判断 {
    /// GitHub Actions 单行输出；版本先经 ASCII SemVer 验证，不能注入其他输出。
    #[must_use]
    pub fn 工作流输出(&self) -> String {
        format!(
            "版本={}\n候选={}\n市场={}\n",
            self.版本, self.候选, self.市场
        )
    }
}

/// 主线缺失、历史不完整或 Git 错误必须失败，不能将未知状态当作正式发布。
pub fn 判断发布(目录: &Path, 引用: &str, 主线: &str) -> 结果<发布判断> {
    let 清单: serde_json::Value =
        serde_json::from_slice(&std::fs::read(目录.join("package.json"))?)
            .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    let 版本 = 清单["version"]
        .as_str()
        .ok_or_else(|| 工具错误::新("扩展版本无效"))?;
    let 解析版本 = semver::Version::parse(版本)
        .map_err(|错误| 工具错误::新(format!("扩展版本无效：{错误}")))?;
    let 标签 = 引用.starts_with("refs/tags/");
    if 标签 && 引用 != format!("refs/tags/v{版本}") {
        return Err(工具错误::新("标签与扩展版本不一致"));
    }
    if 执行git(目录, &["rev-parse", "--is-shallow-repository"])?.trim() == "true" {
        return Err(工具错误::新("判断发布需要完整 Git 历史"));
    }
    let 主线提交 = 执行git(
        目录,
        &["rev-parse", "--verify", &format!("{主线}^{{commit}}")],
    )?;
    let 当前提交 = 执行git(目录, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let 输出 = Command::new("git")
        .arg("-C")
        .arg(目录)
        .args([
            "merge-base",
            "--is-ancestor",
            当前提交.trim(),
            主线提交.trim(),
        ])
        .output()?;
    let 主线包含 = match 输出.status.code() {
        Some(0) => true,
        Some(1) => false,
        _ => {
            return Err(工具错误::新(format!(
                "判断 Git 主线归属失败：{}",
                String::from_utf8_lossy(&输出.stderr).trim()
            )));
        }
    };
    let 候选 = !主线包含 || !解析版本.pre.is_empty();
    Ok(发布判断 {
        版本: 版本.to_string(),
        候选,
        市场: 标签 && !候选,
    })
}

fn 解析参数(参数: &[String]) -> 结果<(String, Option<PathBuf>)> {
    let mut 引用 = None;
    let mut 输出 = None;
    let mut 参数 = 参数.iter();
    while let Some(键) = 参数.next() {
        let 已设置 = match 键.as_str() {
            "--引用" => 引用.is_some(),
            "--输出" => 输出.is_some(),
            _ => return Err(工具错误::新(format!("未知参数：{键}"))),
        };
        if 已设置 {
            return Err(工具错误::新(format!("重复参数：{键}")));
        }
        let 值 = 参数
            .next()
            .filter(|值| !值.is_empty() && !值.starts_with("--"))
            .ok_or_else(|| 工具错误::新(format!("{键} 缺少取值")))?;
        if 键 == "--引用" {
            引用 = Some(值.clone());
        } else {
            输出 = Some(PathBuf::from(值));
        }
    }
    let 引用 = 引用
        .filter(|值| 值.starts_with("refs/") && !值.contains(['\r', '\n']))
        .ok_or_else(|| 工具错误::新("发布策略需要 --引用 refs/..."))?;
    Ok((引用, 输出))
}

/// `adwcode 发布策略 --引用 refs/... [--输出 路径]`；输出文件按 Actions 协议追加。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let (引用, 输出) = 解析参数(参数)?;
    let 策略 = 判断发布(根目录, &引用, "refs/remotes/origin/main")?;
    if let Some(输出) = 输出 {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(输出)?
            .write_all(策略.工作流输出().as_bytes())?;
    }
    println!(
        "版本：{}；候选：{}；市场：{}",
        策略.版本, 策略.候选, 策略.市场
    );
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 新建仓库() -> tempfile::TempDir {
        let 目录 = tempfile::tempdir().unwrap();
        for 参数 in [
            vec!["init", "-b", "main"],
            vec!["config", "user.name", "离线测试"],
            vec!["config", "user.email", "test@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
        ] {
            执行git(目录.path(), &参数).unwrap();
        }
        写版本(目录.path(), "4.1.0");
        执行git(目录.path(), &["add", "package.json"]).unwrap();
        执行git(目录.path(), &["commit", "-m", "主线基线"]).unwrap();
        执行git(
            目录.path(),
            &["update-ref", "refs/remotes/origin/main", "HEAD"],
        )
        .unwrap();
        目录
    }

    fn 写版本(目录: &Path, 版本: &str) {
        std::fs::write(
            目录.join("package.json"),
            serde_json::json!({"version": 版本}).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn 真实仓库覆盖主线分支标签与预发布() {
        let 临时 = 新建仓库();
        let 根 = 临时.path();
        let 判断 = |引用| 判断发布(根, 引用, "refs/remotes/origin/main").unwrap();
        assert_eq!(
            判断("refs/tags/v4.1.0"),
            发布判断 {
                版本: "4.1.0".into(),
                候选: false,
                市场: true
            }
        );
        assert!(!判断("refs/heads/main").市场);
        assert!(判断发布(根, "refs/tags/v4.0.1", "refs/remotes/origin/main").is_err());
        执行git(根, &["checkout", "-b", "rust"]).unwrap();
        执行git(根, &["commit", "--allow-empty", "-m", "分支迭代"]).unwrap();
        assert_eq!(
            判断("refs/tags/v4.1.0"),
            发布判断 {
                版本: "4.1.0".into(),
                候选: true,
                市场: false
            }
        );
        assert!(判断发布(根, "refs/tags/v4.1.0", "refs/heads/不存在").is_err());
        执行git(根, &["update-ref", "refs/remotes/origin/main", "HEAD"]).unwrap();
        assert!(判断("refs/tags/v4.1.0").市场);
        写版本(根, "4.1.0-beta.1+构建");
        assert!(
            判断发布(
                根,
                "refs/tags/v4.1.0-beta.1+构建",
                "refs/remotes/origin/main"
            )
            .is_err()
        );
        写版本(根, "4.1.0-beta.1+build.2");
        assert!(判断("refs/tags/v4.1.0-beta.1+build.2").候选);
        assert!(!判断("refs/tags/v4.1.0-beta.1+build.2").市场);
        写版本(根, "4.1.0+build-test.2");
        assert!(判断("refs/tags/v4.1.0+build-test.2").市场);
        assert!(!判断("refs/heads/rust").市场);
    }

    #[test]
    fn 非法版本与缺失历史失败且不写工作流输出() {
        let 临时 = 新建仓库();
        let 根 = 临时.path();
        for 内容 in [
            r#"{"version":null}"#,
            "{",
            r#"{"version":"04.1.0"}"#,
            r#"{"version":"4.1.0-beta.01"}"#,
            r#"{"version":"4.1.0\n市场=true"}"#,
        ] {
            std::fs::write(根.join("package.json"), 内容).unwrap();
            assert!(判断发布(根, "refs/heads/main", "refs/remotes/origin/main").is_err());
        }
        写版本(根, "4.1.0");
        let 输出 = 根.join("输出");
        std::fs::write(&输出, "已有=保留\n").unwrap();
        let 参数 = vec![
            "--引用".into(),
            "refs/tags/v4.1.0".into(),
            "--输出".into(),
            输出.to_str().unwrap().into(),
        ];
        入口(根, &参数).unwrap();
        assert_eq!(
            std::fs::read_to_string(&输出).unwrap(),
            "已有=保留\n版本=4.1.0\n候选=false\n市场=true\n"
        );
        assert!(
            入口(
                根,
                &[
                    "--引用".into(),
                    "refs/tags/v4.1.0".into(),
                    "--输出".into(),
                    根.to_str().unwrap().into()
                ]
            )
            .is_err()
        );
        执行git(根, &["update-ref", "-d", "refs/remotes/origin/main"]).unwrap();
        let 之前 = std::fs::read(&输出).unwrap();
        assert!(入口(根, &参数).is_err());
        assert_eq!(std::fs::read(&输出).unwrap(), 之前);
        let 提交 = 执行git(根, &["rev-parse", "HEAD"]).unwrap();
        std::fs::write(根.join(".git/shallow"), 提交).unwrap();
        assert!(
            判断发布(根, "refs/heads/main", "HEAD")
                .unwrap_err()
                .消息
                .contains("完整 Git 历史")
        );
        assert!(判断发布(Path::new("/不存在的仓库"), "refs/heads/main", "HEAD").is_err());
        let 非仓库 = tempfile::tempdir().unwrap();
        写版本(非仓库.path(), "4.1.0");
        assert!(判断发布(非仓库.path(), "refs/heads/main", "HEAD").is_err());
    }

    #[test]
    fn 发布参数严格校验() {
        for 参数 in [
            vec![],
            vec!["--引用"],
            vec!["--引用", ""],
            vec!["--引用", "v4.1.0"],
            vec!["--引用", "refs/heads/rust", "多余"],
            vec!["--引用", "refs/heads/rust", "--输出"],
            vec!["--引用", "--输出"],
            vec!["--引用", "refs/heads/rust", "--引用", "refs/heads/main"],
            vec!["--引用", "refs/heads/rust", "--输出", "a", "--输出", "b"],
        ] {
            assert!(解析参数(&参数.into_iter().map(str::to_string).collect::<Vec<_>>()).is_err());
        }
        assert!(解析参数(&["--引用".into(), "refs/heads/rust".into()]).is_ok());
    }
}
