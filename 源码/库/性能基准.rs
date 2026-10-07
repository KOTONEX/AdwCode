// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! Linux 离线性能基准：默认只测墙钟；资源轮询按需启用。
//! 两个预热样本、保留文件缓存、每个样本新进程；临时副本和进程组始终清理。

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use crate::摘要::sha256十六进制;
use crate::错误::{工具错误, 结果};

/// 中位数、最近秩 P95、最小值与最大值。
#[must_use]
pub fn 摘要(值: &[f64]) -> Value {
    if 值.is_empty() || 值.iter().any(|值| !值.is_finite()) {
        return Value::Null;
    }
    let mut 有序 = 值.to_vec();
    有序.sort_by(|左, 右| 左.partial_cmp(右).unwrap_or(std::cmp::Ordering::Equal));
    let 数量 = 有序.len();
    let 中位 = if 数量.is_multiple_of(2) {
        有序[数量 / 2 - 1] / 2.0 + 有序[数量 / 2] / 2.0
    } else {
        有序[数量 / 2]
    };
    let 位次 = ((0.95 * 数量 as f64).ceil() as usize)
        .saturating_sub(1)
        .min(数量.saturating_sub(1));
    json!({
        "median": 中位,
        "p95": 有序[位次],
        "min": 有序[0],
        "max": 有序[数量 - 1],
    })
}

/// Node 只采样真实 JS；沿用报告协议字段，由 Rust 校验并填充摘要。
pub fn 汇总扩展样本(mut 报告: Value) -> 结果<Value> {
    let 场景 = 报告["results"]
        .as_array_mut()
        .filter(|场景| !场景.is_empty())
        .ok_or_else(|| 工具错误::新("扩展基准缺少场景"))?;
    let mut 名称 = std::collections::BTreeSet::new();
    for 场景 in 场景 {
        let 名字 = 场景["name"]
            .as_str()
            .filter(|名字| !名字.is_empty())
            .ok_or_else(|| 工具错误::新("扩展基准场景名称无效"))?;
        if !名称.insert(名字.to_string()) || 场景["unit"] != "ms" {
            return Err(工具错误::新("扩展基准名称重复或单位无效"));
        }
        let 样本 = 场景["samples"]
            .as_array()
            .filter(|样本| !样本.is_empty())
            .ok_or_else(|| 工具错误::新("扩展基准缺少样本"))?;
        if 场景["count"].as_u64() != Some(样本.len() as u64) {
            return Err(工具错误::新("扩展基准样本数量不匹配"));
        }
        let 数值: Vec<f64> = 样本
            .iter()
            .map(|值| {
                值.as_f64()
                    .filter(|值| 值.is_finite() && *值 >= 0.0)
                    .ok_or_else(|| 工具错误::新("扩展基准样本必须为有限非负数"))
            })
            .collect::<结果<_>>()?;
        场景["summary"] = 摘要(&数值);
    }
    Ok(报告)
}

/// 每秒时钟滴答数；读取失败时按常见的 100 处理。
fn 时钟滴答() -> f64 {
    static 缓存: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *缓存.get_or_init(|| {
        Command::new("getconf")
            .arg("CLK_TCK")
            .output()
            .ok()
            .and_then(|输出| String::from_utf8_lossy(&输出.stdout).trim().parse().ok())
            .unwrap_or(100.0)
    })
}

/// `/proc/self/stat` 中已结束子进程的累计 CPU 秒数。
fn 子进程cpu秒() -> 结果<f64> {
    let 文本 = std::fs::read_to_string("/proc/self/stat")?;
    let 尾部 = 文本
        .rsplit_once(')')
        .map(|(_, 尾部)| 尾部)
        .ok_or_else(|| 工具错误::新("/proc/self/stat 格式异常"))?;
    let 字段: Vec<&str> = 尾部.split_whitespace().collect();
    let 取值 = |序号: usize| -> 结果<f64> {
        字段
            .get(序号)
            .and_then(|值| 值.parse::<f64>().ok())
            .ok_or_else(|| 工具错误::新("/proc/self/stat 缺少子进程时间"))
    };
    取值(13).and_then(|用户| 取值(14).map(|系统| (用户 + 系统) / 时钟滴答()))
}

/// 运行一个样本，默认阻塞等待，只测墙钟；详细模式轮询 CPU/RSS，未测量或未采到的值为 null。
fn 采样(命令: &[String], 目录: &Path, 详细资源: bool) -> 结果<Value> {
    let 输出暂存 = tempfile::NamedTempFile::new()?;
    let 输出路径 = 输出暂存.path();
    let 输出文件 = 输出暂存.reopen()?;
    let 可执行 = 详细资源
        .then(|| std::fs::canonicalize(&命令[0]))
        .transpose()?;
    let cpu起 = 详细资源.then(子进程cpu秒).transpose()?;
    let 开始 = Instant::now();
    let mut 子进程 = crate::运行工具::受管进程(
        Command::new(&命令[0])
            .args(&命令[1..])
            .current_dir(目录)
            .stdout(Stdio::from(输出文件.try_clone()?))
            .stderr(Stdio::from(输出文件))
            .process_group(0)
            .spawn()
            .map_err(|错误| {
                工具错误::带来源(format!("无法启动 {}：{错误}", 命令[0]), 错误)
            })?,
    );
    let pid = 子进程.0.id();
    let mut 峰值 = 0u64;
    let 状态 = if !详细资源 {
        子进程.0.wait()?
    } else {
        loop {
            if std::fs::read_link(format!("/proc/{pid}/exe"))
                .ok()
                .as_deref()
                == 可执行.as_deref()
                && let Ok(状态文本) = std::fs::read_to_string(format!("/proc/{pid}/status"))
            {
                for 行 in 状态文本.lines() {
                    if let Some(值) = 行.strip_prefix("VmHWM:")
                        && let Some(数字) = 值.split_whitespace().next()
                        && let Ok(数字) = 数字.parse::<u64>()
                    {
                        峰值 = 峰值.max(数字);
                    }
                }
            }
            if let Some(状态) = 子进程.0.try_wait()? {
                break 状态;
            }
            std::thread::sleep(Duration::from_micros(500));
        }
    };
    let 墙钟 = 开始.elapsed().as_secs_f64() * 1000.0;
    let cpu = cpu起
        .map(|开始| 子进程cpu秒().map(|结束| (结束 - 开始) * 1000.0))
        .transpose()?;
    if 状态.success() {
        Ok(json!({
            "wall_ms": 墙钟,
            "cpu_ms": cpu,
            "rss_mib": (详细资源 && 峰值 > 0).then_some(峰值 as f64 / 1024.0),
        }))
    } else {
        let 内容 = std::fs::read_to_string(输出路径).unwrap_or_default();
        Err(工具错误::新(内容))
    }
}

/// 对源码、主题、字体、扩展、附加外观与清单生成 SHA-256 指纹。
pub fn 指纹(根目录: &Path) -> 结果<Map<String, Value>> {
    let mut 结果 = Map::new();
    for 目录 in ["源码", "主题", "产品图标", "扩展", "附加外观"] {
        let 起点 = 根目录.join(目录);
        let mut 文件: Vec<PathBuf> = Vec::new();
        递归收集(&起点, &mut 文件)?;
        文件.sort_by(|左, 右| 左.to_string_lossy().cmp(&右.to_string_lossy()));
        for 路径 in 文件 {
            let 相对 = 路径
                .strip_prefix(根目录)
                .map_err(|_| 工具错误::新("指纹文件位于仓库之外"))?
                .to_string_lossy()
                .to_string();
            let 内容 = std::fs::read(&路径)?;
            结果.insert(相对, Value::String(sha256十六进制(&内容)));
        }
    }
    let 清单 = std::fs::read(根目录.join("package.json"))?;
    结果.insert(
        "package.json".to_string(),
        Value::String(sha256十六进制(&清单)),
    );
    Ok(结果)
}

fn 递归收集(目录: &Path, 输出: &mut Vec<PathBuf>) -> 结果<()> {
    for 条目 in std::fs::read_dir(目录)? {
        let 路径 = 条目?.path();
        crate::文件事务::校验普通路径(&路径)?;
        if 路径.is_dir() {
            递归收集(&路径, 输出)?;
        } else if 路径.is_file() {
            输出.push(路径);
        }
    }
    Ok(())
}

fn 忽略项(路径: &Path) -> bool {
    路径
        .components()
        .any(|部件| 部件.as_os_str() == "__pycache__")
        || 路径.extension().is_some_and(|扩展| 扩展 == "pyc")
}

fn 版本(根目录: &Path) -> 结果<String> {
    let 文本 = std::fs::read_to_string(根目录.join("package.json"))?;
    let manifest: Value = serde_json::from_str(&文本)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    Ok(manifest["version"].as_str().unwrap_or("").to_string())
}

fn 环境信息() -> Value {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|文本| {
            文本
                .lines()
                .find(|行| 行.starts_with("model name"))
                .and_then(|行| 行.split_once(':').map(|(_, 值)| 值.trim().to_string()))
        })
        .unwrap_or_default();
    let 内存 = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|文本| {
            文本
                .lines()
                .find(|行| 行.starts_with("MemTotal:"))
                .and_then(|行| 行.split_whitespace().nth(1))
                .and_then(|值| 值.parse::<u64>().ok())
        })
        .unwrap_or(0);
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .map(|输出| String::from_utf8_lossy(&输出.stdout).trim().to_string())
        .unwrap_or_default();
    json!({
        "platform": format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        "rustc": rustc,
        "cpu": cpu,
        "memory_kib": 内存,
    })
}

/// `adwcode 性能基准` 的入口。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    if !cfg!(target_os = "linux") {
        return Err(工具错误::新("性能基准只支持 Linux"));
    }
    let mut 次数 = 20usize;
    let mut 详细资源 = false;
    let mut 输出路径 = 根目录.join("builddir/performance.json");
    let mut 序号 = 0;
    while 序号 < 参数.len() {
        match 参数[序号].as_str() {
            "--详细资源" if !详细资源 => {
                详细资源 = true;
            }
            "--次数" => {
                序号 += 1;
                次数 = 参数
                    .get(序号)
                    .and_then(|值| 值.parse().ok())
                    .ok_or_else(|| 工具错误::新("--次数 需要整数取值"))?;
            }
            "--输出" => {
                序号 += 1;
                输出路径 = PathBuf::from(
                    参数
                        .get(序号)
                        .ok_or_else(|| 工具错误::新("--输出 缺少取值"))?,
                );
            }
            其他 => return Err(工具错误::新(format!("未知参数：{其他}"))),
        }
        序号 += 1;
    }
    if 次数 < 5 {
        return Err(工具错误::新("--次数 至少为 5"));
    }
    let before = 指纹(根目录)?;
    let 可执行 = std::env::current_exe()?;
    let 可执行名 = 可执行
        .file_name()
        .map_or_else(String::new, |名字| 名字.to_string_lossy().to_string());
    let 版本号 = 版本(根目录)?;
    let 修订 = crate::变更日志::执行git(根目录, &["rev-parse", "HEAD"])?;
    let mut result = json!({
        "schema": 2,
        "revision": 修订.trim(),
        "version": 版本号,
        "environment": 环境信息(),
        "method": {
            "warmup": 2,
            "runs": 次数,
            "p95": "最近秩：第 ceil(0.95*n) 个有序样本",
            "cache": "保留操作系统文件缓存，每个样本使用新进程",
            "unit": "毫秒；RSS 为 MiB",
            "resources": 详细资源,
            "rss": if 详细资源 { "exec 后每0.5ms采样VmHWM；未采到为null，短进程可能漏掉最终峰值" } else { "未启用资源测量；cpu_ms与rss_mib为null" },
        },
        "commands": [],
    });

    let 临时副本 = tempfile::tempdir()?;
    let 临时根 = 临时副本.path().to_path_buf();
    for 名称 in [
        "源码",
        "测试",
        "类型声明",
        "基准",
        "主题",
        "产品图标",
        "扩展",
        "附加外观",
        "资产",
        "文档",
        "LICENSES",
    ] {
        let 来源 = 根目录.join(名称);
        if 来源.exists() {
            crate::运行工具::复制目录(&来源, &临时根.join(名称), 忽略项)?;
        }
    }
    for 名称 in [
        "package.json",
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "tsconfig.json",
        "README.md",
        "LICENSE",
        "许可声明.md",
        "CONTRIBUTING.md",
        "AGENTS.md",
    ] {
        let 来源 = 根目录.join(名称);
        if 来源.is_file() {
            std::fs::copy(&来源, 临时根.join(名称))?;
        }
    }
    std::fs::create_dir_all(临时根.join("builddir"))?;
    let 日志 = crate::变更日志::生成变更日志(根目录)?;
    std::fs::write(临时根.join("builddir/CHANGELOG.md"), 日志)?;

    let 命令列表: Vec<(&str, Vec<String>)> = vec![
        ("Rust 空进程", vec![可执行.display().to_string()]),
        (
            "默认构建（6 个蓝色主题）",
            vec![可执行.display().to_string(), "主题".to_string()],
        ),
        (
            "默认主题校验",
            vec![可执行.display().to_string(), "校验".to_string()],
        ),
        (
            "CSS 兼容校验",
            vec![可执行.display().to_string(), "检查样式".to_string()],
        ),
        (
            "VSIX 打包",
            vec![
                可执行.display().to_string(),
                "打包".to_string(),
                "--变更日志".to_string(),
                "builddir/CHANGELOG.md".to_string(),
            ],
        ),
    ];
    for (标题, 命令) in &命令列表 {
        if *标题 == "默认主题校验" {
            采样(
                &[可执行.display().to_string(), "主题".to_string()],
                &临时根,
                详细资源,
            )?;
        }
        for _ in 0..2 {
            采样(命令, &临时根, 详细资源)?;
        }
        let mut 样本 = Vec::new();
        for _ in 0..次数 {
            样本.push(采样(命令, &临时根, 详细资源)?);
        }
        let mut 汇总 = Map::new();
        for 键 in ["wall_ms", "cpu_ms", "rss_mib"] {
            let 值: Vec<f64> = 样本.iter().filter_map(|样本| 样本[键].as_f64()).collect();
            汇总.insert(键.to_string(), 摘要(&值));
        }
        let 中位 = 汇总
            .get("wall_ms")
            .and_then(|值| 值["median"].as_f64())
            .unwrap_or(0.0);
        let p95 = 汇总
            .get("wall_ms")
            .and_then(|值| 值["p95"].as_f64())
            .unwrap_or(0.0);
        let rss = 汇总.get("rss_mib").and_then(|值| 值["max"].as_f64());
        let 资源文案 = rss.map_or_else(
            || "RSS 未测量或未采集到".to_owned(),
            |值| format!("观测峰值 RSS {值:.1} MiB"),
        );
        println!("{标题}: 中位 {中位:.2} ms，P95 {p95:.2} ms，{资源文案}");
        result["commands"]
            .as_array_mut()
            .expect("命令数组")
            .push(json!({
                "name": 标题,
                "command": [可执行名.clone(), 命令[1..].to_vec()],
                "samples": 样本,
                "summary": 汇总,
            }));
    }
    if before != 指纹(根目录)? {
        return Err(工具错误::新("基准意外改动了源码或主题"));
    }

    let node = Command::new("node").arg("--version").output().is_ok();
    if !node {
        return Err(工具错误::新("扩展基准需要 Node.js"));
    }
    let 扩展输出 = Command::new("node")
        .arg("--expose-gc")
        .arg(根目录.join("基准/扩展基准.cjs"))
        .arg(&临时根)
        .output()
        .map_err(|错误| 工具错误::带来源("无法运行扩展基准".to_string(), 错误))?;
    if !扩展输出.status.success() {
        return Err(工具错误::新(format!(
            "扩展基准失败：{}",
            String::from_utf8_lossy(&扩展输出.stderr)
        )));
    }
    result["扩展"] = 汇总扩展样本(
        serde_json::from_slice(&扩展输出.stdout)
            .map_err(|错误| 工具错误::新(format!("扩展基准输出解析失败：{错误}")))?,
    )?;
    println!("扩展离线基准完成");
    drop(临时副本);
    if before != 指纹(根目录)? {
        return Err(工具错误::新("基准意外改动了源码或主题"));
    }
    if let Some(父目录) = 输出路径.parent() {
        std::fs::create_dir_all(父目录)?;
    }
    crate::文件事务::写入批次(&[(
        输出路径.clone(),
        Some(crate::主题生成::写json(&result).into_bytes()),
    )])?;
    println!("原始结果：{}", 输出路径.display());
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 摘要求中位数与最近秩p95() {
        assert_eq!(摘要(&[]), Value::Null);
        assert_eq!(摘要(&[f64::NAN]), Value::Null);
        let 值: Vec<f64> = (1..=20).map(f64::from).collect();
        let 结果 = 摘要(&值);
        assert_eq!(结果["median"], 10.5);
        assert_eq!(结果["p95"], 19.0);
        assert_eq!(结果["min"], 1.0);
        assert_eq!(结果["max"], 20.0);
        let 奇数 = 摘要(&[3.0, 1.0, 2.0]);
        assert_eq!(奇数["median"], 2.0);
        assert_eq!(奇数["p95"], 3.0);
        assert_eq!(摘要(&[f64::MAX, f64::MAX])["median"], f64::MAX);
    }

    #[test]
    fn 扩展原始样本汇总保持报告字段并拒绝坏数据() {
        let 原始 = json!({"node": "测试", "results": [{
            "name": "状态读取", "unit": "ms", "count": 4, "samples": [4, 1, 3, 2]
        }]});
        let 汇总 = 汇总扩展样本(原始.clone()).unwrap();
        assert_eq!(汇总["node"], 原始["node"]);
        assert_eq!(汇总["results"][0]["samples"], 原始["results"][0]["samples"]);
        assert_eq!(
            汇总["results"][0]["summary"],
            json!({"median": 2.5, "p95": 4.0, "min": 1.0, "max": 4.0})
        );
        for (键, 值) in [
            ("count", json!(3)),
            ("count", json!(4.5)),
            ("name", json!(null)),
            ("unit", json!("s")),
            ("samples", json!([])),
            ("samples", json!([4, 1, null, 2])),
            ("samples", json!([4, 1, -1, 2])),
            ("samples", json!([4, 1, "3", 2])),
        ] {
            let mut 错误 = 原始.clone();
            错误["results"][0][键] = 值;
            assert!(汇总扩展样本(错误).is_err());
        }
        let mut 重复 = 原始.clone();
        重复["results"]
            .as_array_mut()
            .unwrap()
            .push(原始["results"][0].clone());
        assert!(汇总扩展样本(重复).is_err());
        assert!(汇总扩展样本(json!({"results": []})).is_err());
        assert!(汇总扩展样本(json!({})).is_err());
    }
    #[test]
    fn 墙钟与详细资源模式保留失败处理和空值() {
        let 临时 = tempfile::tempdir().unwrap();
        let 命令 = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "sleep 0.04".to_owned(),
        ];
        let 普通 = 采样(&命令, 临时.path(), false).unwrap();
        assert!(普通["wall_ms"].as_f64().unwrap() >= 20.0);
        assert_eq!(普通["cpu_ms"], Value::Null);
        assert_eq!(普通["rss_mib"], Value::Null);
        let 详细 = 采样(&命令, 临时.path(), true).unwrap();
        assert!(详细["cpu_ms"].as_f64().unwrap() >= 0.0);
        assert!(详细["rss_mib"].as_f64().unwrap() > 0.0);
        let 错误 = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "echo 采样失败 >&2; exit 7".to_owned(),
        ];
        for 模式 in [false, true] {
            assert!(
                采样(&错误, 临时.path(), 模式)
                    .unwrap_err()
                    .to_string()
                    .contains("采样失败")
            );
        }
        assert!(入口(临时.path(), &["--详细资源".into(), "--详细资源".into()]).is_err());
    }
}
