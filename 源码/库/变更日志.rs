// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 按 Git 版本标签和提交标题生成变更日志及发布说明。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::错误::{工具错误, 结果};

/// 预发布标识的 SemVer 片段。
const 预发布: &str = r"(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)";

static 版本标签正则: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-{预发布}(?:\.{预发布})*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    ))
    .expect("版本标签正则")
});

static 转义正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([\\`*_\[\]<>])").expect("转义正则"));

static 仓库地址正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https://github\.com/[\w.-]+/[\w.-]+").expect("仓库地址正则"));

/// 整串完全匹配 SemVer 版本标签。
#[must_use]
pub fn 是版本标签(文本: &str) -> bool {
    版本标签正则
        .find(文本)
        .is_some_and(|匹配| 匹配.as_str() == 文本)
}

/// 中文提交类型分组。
pub const 分组: [&str; 8] = [
    "新增",
    "修复",
    "文档",
    "测试",
    "重构",
    "杂务",
    "初始化",
    "其他",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct 提交记录 {
    pub revision: String,
    pub date: String,
    pub subject: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct 发布节点 {
    pub tag: String,
    pub revision: String,
}

/// 执行只读 Git 命令，失败时保留可诊断的错误。
pub fn 执行git(根目录: &Path, 参数: &[&str]) -> 结果<String> {
    let 输出 = Command::new("git")
        .arg("--no-pager")
        .arg("-C")
        .arg(根目录)
        .args(参数)
        .output()
        .map_err(|错误| 工具错误::带来源(format!("无法运行 Git：{错误}"), 错误))?;
    if !输出.status.success() {
        return Err(工具错误::新(format!(
            "读取 Git 历史失败：{}",
            String::from_utf8_lossy(&输出.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&输出.stdout).to_string())
}

/// 仅使用 HEAD 主线上可达的版本标签，按提交拓扑由新到旧排列。
pub fn 版本节点(根目录: &Path) -> 结果<(String, Vec<发布节点>)> {
    let 顶端文本 = 执行git(根目录, &["rev-parse", "--show-toplevel"])?;
    let 顶端 = std::fs::canonicalize(顶端文本.trim())?;
    let 根 = std::fs::canonicalize(根目录)?;
    if 顶端 != 根 {
        return Err(工具错误::新("生成变更日志需要项目自身的 Git 仓库"));
    }
    if 执行git(根目录, &["rev-parse", "--is-shallow-repository"])?.trim() == "true" {
        return Err(工具错误::新(
            "Git 历史不完整，请获取完整历史和版本标签后重试",
        ));
    }
    let 主线 = 执行git(根目录, &["rev-list", "--first-parent", "HEAD"])?;
    let 主线: Vec<&str> = 主线.lines().collect();
    let 前几个提交 = 主线
        .first()
        .ok_or_else(|| 工具错误::新("Git 仓库尚无提交"))?
        .to_string();
    let 顺序: std::collections::HashMap<&str, usize> = 主线
        .iter()
        .enumerate()
        .map(|(编号, 版本)| (*版本, 编号))
        .collect();
    let 标签文本 = 执行git(
        根目录,
        &[
            "for-each-ref",
            "--merged=HEAD",
            "--format=%(refname:strip=2)",
            "refs/tags",
        ],
    )?;
    let mut 结果列表: Vec<发布节点> = Vec::new();
    let mut 已见: std::collections::HashSet<String> = std::collections::HashSet::new();
    for 标签 in 标签文本.lines() {
        if !是版本标签(标签) {
            continue;
        }
        let 引用 = format!("refs/tags/{标签}^{{commit}}");
        let 提交 = 执行git(根目录, &["rev-parse", 引用.as_str()])?;
        let 提交 = 提交.trim().to_string();
        if !顺序.contains_key(提交.as_str()) {
            continue;
        }
        if !已见.insert(提交.clone()) {
            return Err(工具错误::新(format!(
                "同一提交存在多个版本标签，请核对：{标签}"
            )));
        }
        结果列表.push(发布节点 {
            tag: 标签.to_string(),
            revision: 提交,
        });
    }
    结果列表.sort_by_key(|节点| 顺序[节点.revision.as_str()]);
    Ok((前几个提交, 结果列表))
}

/// 读取版本范围内的非合并提交，包括合入主线的分支提交。
pub fn 提交列表(
    根目录: &Path, 结束: &str, 起始: Option<&str>
) -> 结果<Vec<提交记录>> {
    let 范围 = match 起始 {
        Some(起始) => format!("{起始}..{结束}"),
        None => 结束.to_string(),
    };
    let 文本 = 执行git(
        根目录,
        &[
            "log",
            "--no-show-signature",
            "--no-merges",
            "--topo-order",
            "--reverse",
            "-z",
            "--format=%H%x00%cs%x00%s",
            范围.as_str(),
            "--",
        ],
    )?;
    let mut 字段: Vec<&str> = 文本.split('\0').collect();
    if 字段.last() == Some(&"") {
        字段.pop();
    }
    if 字段.len() % 3 != 0 {
        return Err(工具错误::新("Git 提交记录格式不完整"));
    }
    Ok(字段
        .chunks(3)
        .map(|块| 提交记录 {
            revision: 块[0].to_string(),
            date: 块[1].to_string(),
            subject: 块[2].to_string(),
        })
        .collect())
}

/// 将提交标题作为普通 Markdown 文本展示。
#[must_use]
pub fn 转义文本(文本: &str) -> String {
    转义正则.replace_all(文本, "\\$1").to_string()
}

/// 按项目中文提交类型分组，未分类标题仍完整保留。
#[must_use]
pub fn 渲染提交列表(条目: &[提交记录]) -> String {
    let mut 分组内容: Vec<Vec<String>> = vec![Vec::new(); 分组.len()];
    for 提交 in 条目 {
        let (前缀, 描述) = match 提交.subject.split_once(": ") {
            Some((前缀, 描述)) => (前缀, 描述),
            None => ("", ""),
        };
        let 组 = if !描述.is_empty() && 分组[..分组.len() - 1].contains(&前缀) {
            前缀
        } else {
            "其他"
        };
        let 标题 = if 组 != "其他" {
            描述
        } else {
            &提交.subject
        };
        let 编号 = 分组.iter().position(|名称| *名称 == 组).expect("分组存在");
        let 短修订 = &提交.revision[..提交.revision.len().min(7)];
        分组内容[编号].push(format!("- {}（`{短修订}`）", 转义文本(标题)));
    }
    let mut 段落: Vec<String> = Vec::new();
    for (编号, 名称) in 分组.iter().enumerate() {
        if !分组内容[编号].is_empty() {
            段落.push(format!("### {名称}\n\n{}", 分组内容[编号].join("\n")));
        }
    }
    段落.join("\n\n")
}

/// 生成完整日志；未提交修改不属于 Git 提交历史。
pub fn 生成变更日志(根目录: &Path) -> 结果<String> {
    let (头, 版本列表) = 版本节点(根目录)?;
    let 未发布 = 提交列表(
        根目录,
        &头,
        版本列表.first().map(|节点| 节点.revision.as_str()),
    )?;
    let mut 段落 = vec![
        "# 变更日志".to_string(),
        "<!-- 由 adwcode 变更日志 根据 Git 历史自动生成，请勿手工编辑。 -->".to_string(),
        "记录版本范围内的非合并提交标题；未提交修改不在本日志中。".to_string(),
        format!(
            "## [未发布]\n\n{}",
            if 未发布.is_empty() {
                "尚无新增提交。".to_string()
            } else {
                渲染提交列表(&未发布)
            }
        ),
    ];
    for (序号, 发布) in 版本列表.iter().enumerate() {
        let 起始 = 版本列表.get(序号 + 1).map(|节点| 节点.revision.as_str());
        let 条目 = 提交列表(根目录, &发布.revision, 起始)?;
        let 日期 = 执行git(
            根目录,
            &[
                "show",
                "--no-show-signature",
                "-s",
                "--format=%cs",
                发布.revision.as_str(),
            ],
        )?;
        let 标签 = 发布.tag.strip_prefix('v').unwrap_or(&发布.tag);
        段落.push(format!(
            "## [{标签}] - {}\n\n{}",
            日期.trim(),
            if 条目.is_empty() {
                "此范围仅包含合并提交。".to_string()
            } else {
                渲染提交列表(&条目)
            }
        ));
    }
    Ok(段落.join("\n\n") + "\n")
}

/// 生成当前版本说明；发布时要求版本标签严格对应 HEAD 与清单版本。
pub fn 生成发布说明(根目录: &Path, version: &str, tag: Option<&str>) -> 结果<String> {
    if !是版本标签(&format!("v{version}")) {
        return Err(工具错误::新(
            "package.json 的版本号不是有效的版本标签格式",
        ));
    }
    let (头, 版本列表) = 版本节点(根目录)?;
    let 当前 = 版本列表
        .iter()
        .find(|发布| 发布.tag == format!("v{version}"));
    if let Some(标签) = tag {
        if 标签 != format!("v{version}") {
            return Err(工具错误::新("发布标签与 package.json 的版本不一致"));
        }
        if 当前.is_none_or(|发布| 发布.revision != 头) {
            return Err(工具错误::新("发布标签必须存在且指向当前 HEAD"));
        }
    }
    let (上一发布, 标题) = if 版本列表.first().is_some_and(|发布| 发布.revision == 头) {
        if 版本列表[0].tag != format!("v{version}") {
            return Err(工具错误::新("HEAD 的版本标签与 package.json 不一致"));
        }
        (版本列表.get(1), format!("## [{version}]"))
    } else {
        (版本列表.first(), format!("## [{version}]（未打标签预览）"))
    };
    let 条目 = 提交列表(根目录, &头, 上一发布.map(|发布| 发布.revision.as_str()))?;
    if 条目.is_empty() {
        return Err(工具错误::新(
            "当前版本范围内没有非合并提交，无法生成发布说明",
        ));
    }
    let mut 说明 = 标题 + "\n\n" + &渲染提交列表(&条目) + "\n";
    if tag.is_some()
        && let Some(上一发布) = 上一发布
    {
        let 清单文本 = std::fs::read_to_string(根目录.join("package.json"))?;
        let manifest: serde_json::Value = serde_json::from_str(&清单文本)
            .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
        let 地址 = manifest
            .get("repository")
            .and_then(|仓库| 仓库.get("url").or(Some(仓库)))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let 地址 = 地址
            .trim_end_matches('/')
            .strip_suffix(".git")
            .unwrap_or(地址.trim_end_matches('/'));
        if 仓库地址正则
            .find(地址)
            .is_some_and(|匹配| 匹配.as_str() == 地址)
        {
            let 原始标签 = tag.unwrap_or_default();
            let 链接 = format!(
                "{地址}/compare/{}...{}",
                quote(&上一发布.tag),
                quote(原始标签)
            );
            说明 += &format!("\n**完整变更**：[{} → {原始标签}]({链接})\n", 上一发布.tag);
        }
    }
    Ok(说明)
}

/// 与 Python `urllib.parse.quote`（默认 safe="/"）一致的最小百分号编码。
#[must_use]
pub fn quote(文本: &str) -> String {
    let mut 结果 = String::new();
    for 字节 in 文本.bytes() {
        if 字节.is_ascii_alphanumeric() || b"/_.-~".contains(&字节) {
            结果.push(字节 as char);
        } else {
            结果 += &format!("%{字节:02X}");
        }
    }
    结果
}

/// `adwcode 变更日志` 与 `adwcode 发布说明` 的入口。
pub fn 入口(根目录: &Path, 是发布说明: bool, 参数: &[String]) -> 结果<()> {
    let mut 标签: Option<String> = None;
    let mut 输出: Option<PathBuf> = None;
    let mut 序号 = 0;
    while 序号 < 参数.len() {
        match 参数[序号].as_str() {
            "--标签" => {
                序号 += 1;
                标签 = Some(
                    参数
                        .get(序号)
                        .ok_or_else(|| 工具错误::新("--标签 缺少取值"))?
                        .clone(),
                );
            }
            "--输出" => {
                序号 += 1;
                输出 = Some(PathBuf::from(
                    参数
                        .get(序号)
                        .ok_or_else(|| 工具错误::新("--输出 缺少取值"))?,
                ));
            }
            其他 => return Err(工具错误::新(format!("未知参数：{其他}"))),
        }
        序号 += 1;
    }
    if 标签.is_some() && !是发布说明 {
        return Err(工具错误::新("--标签 仅用于发布说明"));
    }
    let 内容 = if 是发布说明 {
        let 清单文本 = std::fs::read_to_string(根目录.join("package.json"))?;
        let manifest: serde_json::Value = serde_json::from_str(&清单文本)
            .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
        let 版本 = manifest["version"]
            .as_str()
            .ok_or_else(|| 工具错误::新("package.json 缺少 version"))?;
        生成发布说明(根目录, 版本, 标签.as_deref())?
    } else {
        生成变更日志(根目录)?
    };
    let 路径 = 输出.unwrap_or_else(|| {
        根目录.join(if 是发布说明 {
            "builddir/release-notes.md"
        } else {
            "builddir/CHANGELOG.md"
        })
    });
    if let Some(父目录) = 路径.parent() {
        std::fs::create_dir_all(父目录)?;
    }
    std::fs::write(&路径, 内容)?;
    println!("已生成 {}", 路径.display());
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 转义markdown字符() {
        assert_eq!(
            转义文本("修复 `代码` 与 [链接]"),
            "修复 \\`代码\\` 与 \\[链接\\]"
        );
        assert_eq!(转义文本("普通标题"), "普通标题");
    }

    #[test]
    fn 按中文类型分组渲染() {
        let 条目 = vec![
            提交记录 {
                revision: "abcdef0123".to_string(),
                date: "2026-10-06".to_string(),
                subject: "新增: 添加主题".to_string(),
            },
            提交记录 {
                revision: "1234567890".to_string(),
                date: "2026-10-06".to_string(),
                subject: "修复: 修正颜色".to_string(),
            },
            提交记录 {
                revision: "9876543210".to_string(),
                date: "2026-10-06".to_string(),
                subject: "没有类型前缀".to_string(),
            },
        ];
        let 文本 = 渲染提交列表(&条目);
        assert!(文本.starts_with("### 新增"));
        assert!(文本.contains("- 添加主题（`abcdef0`）"));
        assert!(文本.contains("### 修复"));
        assert!(文本.contains("- 修正颜色（`1234567`）"));
        assert!(文本.contains("### 其他"));
        assert!(文本.contains("- 没有类型前缀（`9876543`）"));
        assert!(!文本.contains("### 文档"));
    }

    #[test]
    fn 版本标签与百分号编码() {
        assert!(是版本标签("v3.3.0"));
        assert!(是版本标签("v1.0.0-rc.1+build.2"));
        assert!(!是版本标签("3.3.0"));
        assert!(!是版本标签("v01.0.0"));
        assert!(!是版本标签("v1.0.0ext"));
        assert_eq!(quote("v3.3.0"), "v3.3.0");
        assert_eq!(quote("a b/c+"), "a%20b/c%2B");
    }

    #[test]
    fn 当前仓库生成日志并包含版本标题() {
        let 根 = Path::new(env!("CARGO_MANIFEST_DIR"));
        let 文本 = 生成变更日志(根).expect("生成变更日志");
        assert!(文本.starts_with("# 变更日志\n"));
        assert!(文本.contains("## [未发布]"));
        assert!(文本.contains("## [3.3.0] - "));
    }
}
