// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 有超时和脱敏诊断的发布子进程；临时文件避免标准管道阻塞。

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::运行工具::受管进程;
use crate::错误::{工具错误, 结果};

pub(crate) struct 命令输出 {
    pub 成功: bool,
    pub 退出码: Option<i32>,
    pub 标准输出: String,
    pub 诊断: String,
}

fn 脱敏(文本: &str, 敏感值: &[String]) -> String {
    let 转义 = regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").expect("固定正则");
    let 文本 = 转义.replace_all(文本, "");
    let mut 文本: String = 文本
        .chars()
        .filter(|字符| !字符.is_control() || matches!(字符, '\n' | '\t'))
        .collect();
    let mut 令牌: Vec<_> = 敏感值.iter().filter(|值| !值.is_empty()).collect();
    令牌.sort_by_key(|值| std::cmp::Reverse(值.len()));
    for 值 in 令牌 {
        文本 = 文本.replace(值.as_str(), "<已脱敏>");
        // 读取上限可能截在令牌中间，末尾匹配的前缀也必须遮蔽。
        if let Some(长度) = (1..值.len())
            .rev()
            .find(|长度| 值.is_char_boundary(*长度) && 文本.ends_with(&值[..*长度]))
        {
            文本.truncate(文本.len() - 长度);
            文本.push_str("<已脱敏>");
        }
    }
    let 摘要: String = 文本.chars().take(4096).collect();
    if 文本.chars().count() > 4096 {
        format!("{摘要}…（诊断已截短）")
    } else {
        摘要
    }
}

fn 读取诊断(文件: &mut File, 敏感值: &[String]) -> 结果<String> {
    文件.seek(SeekFrom::Start(0))?;
    let 上限 = 65536 + 敏感值.iter().map(String::len).max().unwrap_or(0);
    let mut 字节 = Vec::new();
    文件.take(上限 as u64).read_to_end(&mut 字节)?;
    Ok(脱敏(&String::from_utf8_lossy(&字节), 敏感值))
}

pub(crate) fn 运行(
    目录: &Path,
    阶段: &str,
    程序: &str,
    参数: &[String],
    限时: Duration,
    敏感值: &[String],
) -> 结果<命令输出> {
    let mut 标准输出 = tempfile::tempfile()?;
    let mut 标准错误 = tempfile::tempfile()?;
    let 子进程 = Command::new(程序)
        .args(参数)
        .current_dir(目录)
        .stdin(Stdio::null())
        .stdout(标准输出.try_clone()?)
        .stderr(标准错误.try_clone()?)
        .process_group(0)
        .spawn()
        .map_err(|错误| 工具错误::新(format!("{阶段} 启动失败：{错误}")))?;
    let mut 子进程 = 受管进程(子进程);
    let 开始 = Instant::now();
    let 状态 = loop {
        if let Some(状态) = 子进程
            .0
            .try_wait()
            .map_err(|错误| 工具错误::新(format!("{阶段} 等待失败：{错误}")))?
        {
            break Some(状态);
        }
        if 开始.elapsed() >= 限时 {
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    drop(子进程);
    let 诊断 = 读取诊断(&mut 标准错误, 敏感值)?;
    let Some(状态) = 状态 else {
        return Err(工具错误::新(format!(
            "{阶段} 超时（{} 秒）；进程组已清理；{诊断}",
            限时.as_secs_f64()
        )));
    };
    标准输出.seek(SeekFrom::Start(0))?;
    let mut 字节 = Vec::new();
    标准输出.take(8 * 1024 * 1024 + 1).read_to_end(&mut 字节)?;
    if 字节.len() > 8 * 1024 * 1024 {
        return Err(工具错误::新(
            format!("{阶段} 标准输出超过 8 MiB，停止发布"),
        ));
    }
    Ok(命令输出 {
        成功: 状态.success(),
        退出码: 状态.code(),
        标准输出: String::from_utf8_lossy(&字节).into_owned(),
        诊断,
    })
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 参数(文本: &str) -> Vec<String> {
        vec!["-c".into(), 文本.into()]
    }

    #[test]
    fn 失败保留退出码并脱敏完整截断及转义令牌() {
        let 临时 = tempfile::tempdir().unwrap();
        let 密钥 = vec!["假令牌abc123".into()];
        let 输出 = 运行(
            临时.path(),
            "测试阶段",
            "sh",
            &参数("printf '假令牌abc123 网络失败' >&2; exit 7"),
            Duration::from_secs(2),
            &密钥,
        )
        .unwrap();
        assert!(!输出.成功);
        assert_eq!(输出.退出码, Some(7));
        assert!(输出.诊断.contains("网络失败"));
        assert!(!输出.诊断.contains("abc123"));
        assert_eq!(脱敏("\u{1b}[31m假令牌abc123\u{1b}[0m", &密钥), "<已脱敏>");
        assert_eq!(脱敏("错误 假令牌abc", &密钥), "错误 <已脱敏>");
        assert!(
            运行(
                临时.path(),
                "缺失工具阶段",
                "/不存在的发布程序",
                &[],
                Duration::from_secs(1),
                &[]
            )
            .err()
            .unwrap()
            .消息
            .contains("缺失工具阶段")
        );
    }

    #[test]
    fn 大量诊断不阻塞且标准输出保持完整() {
        let 临时 = tempfile::tempdir().unwrap();
        let 输出 = 运行(
            临时.path(),
            "大输出",
            "sh",
            &参数("printf '%0200000d' 0 >&2; printf '{\"ok\":true}'"),
            Duration::from_secs(2),
            &[],
        )
        .unwrap();
        assert!(输出.成功);
        assert_eq!(输出.标准输出, "{\"ok\":true}");
        assert!(输出.诊断.len() < 13000);
    }

    #[test]
    fn 超时诊断带阶段并清理忽略终止信号的后代() {
        let 临时 = tempfile::tempdir().unwrap();
        let 标记 = 临时.path().join("不应写入");
        let 参数 = vec![
            "-c".into(),
            "trap '' TERM; (sleep 2; touch \"$1\") & printf '假密钥' >&2; sleep 20".into(),
            "sh".into(),
            标记.to_string_lossy().into_owned(),
        ];
        let 错误 = 运行(
            临时.path(),
            "上传附件",
            "sh",
            &参数,
            Duration::from_millis(80),
            &["假密钥".into()],
        )
        .err()
        .unwrap();
        assert!(错误.消息.contains("上传附件 超时"));
        assert!(!错误.消息.contains("假密钥"));
        std::thread::sleep(Duration::from_millis(1100));
        assert!(!标记.exists());
    }
}
