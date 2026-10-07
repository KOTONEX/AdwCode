// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 生成 AdwCode 主题并同步 package.json。

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value};

use crate::错误::{工具错误, 结果};
use crate::{界面映射, 语法映射, 调色板};

/// `构建计划()` 生成的一项主题构建请求。
pub struct 主题构建请求 {
    pub mode: &'static str,
    pub accent: &'static str,
    pub variant: &'static str,
    pub hc: bool,
}

/// `package.json` 中 `contributes.themes` 的一个条目。
#[derive(Clone)]
pub struct 主题注册项 {
    pub label: String,
    pub ui_theme: String,
    pub path: String,
    pub watch: bool,
}

/// 主题文件名使用中文，保留项目的 adwcode 前缀。
#[must_use]
pub fn 主题文件名(mode: &str, _accent: &str, variant: &str, high_contrast: bool) -> String {
    let mut parts = vec!["adwcode".to_string()];
    parts.push(调色板::模式标签(mode).expect("模式已校验").to_string());
    if high_contrast {
        parts.push("高对比度".to_string());
    } else if variant == "colorful" {
        parts.push("彩色状态栏".to_string());
    }
    parts.join("-") + ".json"
}

fn 相对亮度(颜色: &str) -> 结果<f64> {
    let (r, g, b, _) = 调色板::解析颜色(颜色)?;
    let 通道 = |值: f64| {
        let 值 = 值 / 255.0;
        if 值 <= 0.039_28 {
            值 / 12.92
        } else {
            ((值 + 0.055) / 1.055).powf(2.4)
        }
    };
    Ok(0.2126 * 通道(f64::from(r)) + 0.7152 * 通道(f64::from(g)) + 0.0722 * 通道(f64::from(b)))
}

pub fn 对比度(颜色a: &str, 颜色b: &str) -> 结果<f64> {
    let 可见前景 = 调色板::叠加颜色(颜色a, 颜色b)?;
    let 亮度a = 相对亮度(&可见前景)?;
    let 亮度b = 相对亮度(颜色b)?;
    let 较亮 = 亮度a.max(亮度b);
    let 较暗 = 亮度a.min(亮度b);
    Ok((较亮 + 0.05) / (较暗 + 0.05))
}

#[must_use]
pub fn 主题标签(mode: &str) -> String {
    format!("AdwCode {}", 调色板::模式标签(mode).expect("模式已校验"))
}

/// 生成单个主题对象；`variant` 取值：builder、colorful。
pub fn 生成主题对象(根目录: &Path, 请求: &主题构建请求) -> 结果<Value> {
    let scheme = 语法映射::编辑器颜色(根目录, 请求.mode)?;
    let 调色板 = 调色板::调色板对象::新(请求.mode, 请求.accent, 请求.hc, scheme)?;
    if 变体无效(请求.variant) {
        return Err(工具错误::新(format!("未知主题变体：{}", 请求.variant)));
    }
    let colorful = 请求.variant == "colorful";
    let label = if 请求.hc {
        format!(
            "AdwCode {} 高对比度",
            调色板::模式标签(请求.mode).expect("模式已校验")
        )
    } else {
        let mut label = 主题标签(请求.mode);
        if colorful {
            label.push_str(" · 彩色状态栏");
        }
        label
    };
    let colors = 界面映射::生成界面颜色(&调色板, colorful, 根目录)?;
    let token = 语法映射::语法颜色(根目录, 请求.mode)?;
    let semantic = 语法映射::语义标记颜色(根目录, 请求.mode)?;

    let mut 对象 = Map::new();
    对象.insert(
        "$schema".to_string(),
        Value::String("vscode://schemas/color-theme".to_string()),
    );
    对象.insert("name".to_string(), Value::String(label));
    对象.insert("type".to_string(), Value::String(请求.mode.to_string()));
    对象.insert("semanticHighlighting".to_string(), Value::Bool(true));
    let mut 颜色表 = Map::new();
    for (键, 值) in colors.条目() {
        颜色表.insert(键.clone(), Value::String(值.clone()));
    }
    对象.insert("colors".to_string(), Value::Object(颜色表));
    let mut 规则表 = Vec::new();
    for 规则 in token {
        let mut 设置 = Map::new();
        for (键, 值) in 规则.settings.条目() {
            设置.insert(键.clone(), Value::String(值.clone()));
        }
        let mut 规则对象 = Map::new();
        规则对象.insert(
            "scope".to_string(),
            Value::Array(规则.scope.into_iter().map(Value::String).collect()),
        );
        规则对象.insert("settings".to_string(), Value::Object(设置));
        规则表.push(Value::Object(规则对象));
    }
    对象.insert("tokenColors".to_string(), Value::Array(规则表));
    let mut 语义表 = Map::new();
    for (名称, 值) in semantic {
        语义表.insert(
            名称,
            match 值 {
                Some(值) => Value::String(值),
                None => Value::Null,
            },
        );
    }
    对象.insert("semanticTokenColors".to_string(), Value::Object(语义表));
    Ok(Value::Object(对象))
}

fn 变体无效(variant: &str) -> bool {
    !matches!(variant, "builder" | "colorful")
}

#[must_use]
pub fn 构建计划(accents: &[&'static str]) -> Vec<主题构建请求> {
    let mut plan = Vec::new();
    for mode in ["dark", "light"] {
        for accent in accents {
            for variant in ["builder", "colorful"] {
                plan.push(主题构建请求 {
                    mode,
                    accent,
                    variant,
                    hc: false,
                });
            }
        }
        plan.push(主题构建请求 {
            mode,
            accent: "blue",
            variant: "builder",
            hc: true,
        });
    }
    plan
}

/// 生成全部注册项与主题对象但不写盘；生成器失败时不产生任何文件。
fn 准备主题(
    根目录: &Path,
    plan: &[主题构建请求],
    watch: bool,
) -> 结果<Vec<(String, Value, 主题注册项)>> {
    let mut prepared = Vec::new();
    for 请求 in plan {
        let 主题 = 生成主题对象(根目录, 请求)?;
        let 文件名 = 主题文件名(请求.mode, 请求.accent, 请求.variant, 请求.hc);
        let ui_theme = if 请求.hc {
            if 请求.mode == "dark" {
                "hc-black"
            } else {
                "hc-light"
            }
        } else if 请求.mode == "dark" {
            "vs-dark"
        } else {
            "vs"
        };
        let 注册项 = 主题注册项 {
            label: 主题["name"].as_str().expect("主题名").to_string(),
            ui_theme: ui_theme.to_string(),
            path: format!("./主题/{文件名}"),
            watch,
        };
        prepared.push((文件名, 主题, 注册项));
    }
    Ok(prepared)
}

/// 只生成注册项，不写盘；供清单同步与测试使用。
pub fn 注册项(
    根目录: &Path,
    plan: &[主题构建请求],
    watch: bool,
) -> 结果<Vec<主题注册项>> {
    Ok(准备主题(根目录, plan, watch)?
        .into_iter()
        .map(|(_, _, 注册项)| 注册项)
        .collect())
}

/// 写入全部主题并返回注册项；先生成所有内容，失败时保留原有主题文件。
pub fn 写入主题(
    根目录: &Path,
    plan: &[主题构建请求],
    watch: bool,
) -> 结果<Vec<主题注册项>> {
    let 主题目录 = 根目录.join("主题");
    std::fs::create_dir_all(&主题目录)?;
    let prepared = 准备主题(根目录, plan, watch)?;
    let mut 操作 = Vec::new();
    let mut written: HashSet<String> = HashSet::new();
    let mut entries = Vec::new();
    for (文件名, 主题, 注册项) in prepared {
        操作.push((主题目录.join(&文件名), Some(写json(&主题).into_bytes())));
        written.insert(文件名);
        entries.push(注册项);
    }
    for 旧文件 in std::fs::read_dir(&主题目录)? {
        let 旧文件 = 旧文件?;
        let 名字 = 旧文件.file_name().to_string_lossy().to_string();
        if 名字.starts_with("adwcode-") && 名字.ends_with(".json") && !written.contains(&名字)
        {
            操作.push((旧文件.path(), None));
        }
    }
    操作.push((
        根目录.join("package.json"),
        Some(清单内容(根目录, &entries)?),
    ));
    crate::文件事务::写入批次(&操作)?;
    Ok(entries)
}

/// 将 `contributes.themes` 写回 `package.json`。
pub fn 更新扩展清单(根目录: &Path, entries: &[主题注册项]) -> 结果<()> {
    crate::文件事务::写入批次(&[(
        根目录.join("package.json"),
        Some(清单内容(根目录, entries)?),
    )])
}

fn 清单内容(根目录: &Path, entries: &[主题注册项]) -> 结果<Vec<u8>> {
    let 路径 = 根目录.join("package.json");
    let 文本 = std::fs::read_to_string(&路径)?;
    let mut manifest: Value = serde_json::from_str(&文本)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    let 列表 = entries
        .iter()
        .map(|条目| {
            let mut 对象 = Map::new();
            对象.insert("label".to_string(), Value::String(条目.label.clone()));
            对象.insert("uiTheme".to_string(), Value::String(条目.ui_theme.clone()));
            对象.insert("path".to_string(), Value::String(条目.path.clone()));
            if 条目.watch {
                对象.insert("_watch".to_string(), Value::Bool(true));
            }
            Value::Object(对象)
        })
        .collect();
    manifest["contributes"]["themes"] = Value::Array(列表);
    Ok(写json(&manifest).into_bytes())
}

/// 与 Python `json.dumps(obj, indent=2, ensure_ascii=False) + "\n"` 对齐。
#[must_use]
pub fn 写json(值: &Value) -> String {
    let mut 文本 = serde_json::to_string_pretty(值).expect("JSON 序列化");
    文本.push('\n');
    文本
}

/// 单个颜色键集合；文件缺失时为 `None`。
pub type 颜色键表 = Option<HashSet<String>>;

/// 返回（注册表键集合, 内置主题键集合）。
pub fn 加载颜色键表(根目录: &Path) -> 结果<(颜色键表, 颜色键表)> {
    let 数据目录 = 根目录.join("源码").join("VSCode默认数据");
    let 读取 = |名字: &str| -> 结果<Option<HashSet<String>>> {
        let 路径 = 数据目录.join(名字);
        if !路径.exists() {
            return Ok(None);
        }
        let 文本 = std::fs::read_to_string(&路径)?;
        let 值: Value = serde_json::from_str(&文本)
            .map_err(|错误| 工具错误::新(format!("{名字} 解析失败：{错误}")))?;
        let 集合 = 值
            .as_array()
            .ok_or_else(|| 工具错误::新(format!("{名字} 应为数组")))?
            .iter()
            .map(|项| {
                项.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| 工具错误::新(format!("{名字} 包含非字符串键")))
            })
            .collect::<结果<HashSet<String>>>()?;
        Ok(Some(集合))
    };
    Ok((读取("registry_keys.json")?, 读取("builtin_keys.json")?))
}

/// 有效但未列入随附注册表的键；不包含已弃用的兼容别名。
#[must_use]
pub fn 补充颜色键() -> HashSet<&'static str> {
    [
        "contrastActiveBorder",
        "editorHoverWidget.highlightForeground",
        "editorSuggestWidget.selectedBackground",
        "editorWidget.shadow",
        "editorWidget.errorBorder",
        "editorWidget.warningBorder",
        "editorWidget.infoBorder",
        "editorWidget.prominentBackground",
        "editorWidget.prominentForeground",
        "editorWidget.prominentBorder",
        "extensionButton.prominentBorder",
        "editorPlaceholder.foreground",
        "scm.providerBorder",
    ]
    .into_iter()
    .collect()
}

fn 是有效颜色(值: Option<&Value>) -> bool {
    let Some(Value::String(文本)) = 值 else {
        return false;
    };
    (文本.len() == 7 || 文本.len() == 9)
        && 文本.starts_with('#')
        && 文本[1..].chars().all(|字符| 字符.is_ascii_hexdigit())
}

fn 是字形码点(文本: &str) -> bool {
    let Some(十六进制) = 文本.strip_prefix('\\') else {
        return false;
    };
    (4..=6).contains(&十六进制.len())
        && 十六进制.chars().all(|字符| 字符.is_ascii_hexdigit())
        && u32::from_str_radix(十六进制, 16).is_ok_and(|值| char::from_u32(值).is_some())
}

/// 规范化路径，仅做词法上的 `.` / `..` 折叠，不做符号链接解析。
#[must_use]
pub fn 规范路径(路径: &Path) -> PathBuf {
    let mut 结果 = PathBuf::new();
    for 部件 in 路径.components() {
        match 部件 {
            Component::CurDir => {}
            Component::ParentDir => {
                结果.pop();
            }
            其他 => 结果.push(其他),
        }
    }
    结果
}

/// 校验已生成的主题、产品图标与颜色键；返回失败项数量。
pub fn 校验(根目录: &Path) -> 结果<i32> {
    let mut failures = 0;
    let (registry, builtin) = 加载颜色键表(根目录)?;
    let (Some(registry), Some(builtin)) = (registry, builtin) else {
        println!("失败：缺少 VS Code 颜色键表，请运行 adwcode 更新默认数据 刷新数据");
        return Ok(1);
    };
    主题目录检查(根目录, &registry, &builtin, &mut failures)
}

fn 主题目录检查(
    根目录: &Path,
    registry: &HashSet<String>,
    builtin: &HashSet<String>,
    failures: &mut i32,
) -> 结果<i32> {
    let 主题目录 = 根目录.join("主题");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&主题目录)?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|项| 项.path())
        .filter(|路径| {
            路径
                .extension()
                .is_some_and(|扩展| 扩展.eq_ignore_ascii_case("json"))
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        println!("失败：没有可校验的主题，请先构建主题");
        return Ok(1);
    }
    let 清单文本 = std::fs::read_to_string(根目录.join("package.json"))?;
    let manifest: Value = serde_json::from_str(&清单文本)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    let 注册列表 = manifest["contributes"]["themes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut registered: Vec<(PathBuf, &Value)> = Vec::new();
    for entry in &注册列表 {
        if let Some(路径文本) = entry["path"].as_str() {
            registered.push((规范路径(&根目录.join(路径文本)), entry));
        }
    }
    let 注册集合: HashSet<PathBuf> = registered.iter().map(|(路径, _)| 路径.clone()).collect();
    let 文件集合: HashSet<PathBuf> = paths.iter().map(|路径| 规范路径(路径)).collect();
    if 注册集合.len() != 注册列表.len() || 注册集合 != 文件集合 {
        println!("失败：主题文件与 package.json 注册列表不一致，存在缺失、重复或未注册文件");
        *failures += 1;
    }

    let mut our_keys: HashSet<String> = HashSet::new();
    let mut labels: Vec<String> = Vec::new();
    let mut failed_themes = 0;
    let mut 通过主题 = 0;
    for path in &paths {
        let 文本 = std::fs::read_to_string(path)?;
        let theme: Value = serde_json::from_str(&文本)
            .map_err(|错误| 工具错误::新(format!("{} 解析失败：{错误}", path.display())))?;
        let 文件名 = path
            .file_name()
            .map_or_else(String::new, |名字| 名字.to_string_lossy().to_string());
        let 类型 = theme["type"].as_str().unwrap_or("");
        if 类型 != "dark" && 类型 != "light" {
            println!("失败 {文件名}：未知主题类型 {类型}");
            *failures += 1;
        }
        let 规范 = 规范路径(path);
        if let Some((_, entry)) = registered.iter().find(|(已有, _)| 已有 == &规范) {
            let 期望主题 = if 类型 == "dark" {
                ["vs-dark", "hc-black"].as_slice()
            } else {
                ["vs", "hc-light"].as_slice()
            };
            let ui = entry["uiTheme"].as_str().unwrap_or("");
            if entry["label"].as_str() != theme["name"].as_str() || !期望主题.contains(&ui) {
                println!("失败 {文件名}：主题标签或明暗类型与注册信息不一致");
                *failures += 1;
            }
        }
        if theme["name"].as_str().is_none_or(str::is_empty)
            || theme["colors"].as_object().is_none_or(|表| 表.is_empty())
            || theme["tokenColors"].as_array().is_none_or(Vec::is_empty)
            || theme["semanticTokenColors"]
                .as_object()
                .is_none_or(|表| 表.is_empty())
        {
            println!("失败 {文件名}：缺少名称、颜色表、语法规则或语义颜色表");
            *failures += 1;
        }
        labels.push(theme["name"].as_str().unwrap_or("").to_string());
        if let Some(颜色表) = theme["colors"].as_object() {
            let 缺失: Vec<_> = builtin
                .iter()
                .filter(|键| {
                    !颜色表.contains_key(*键)
                        && !(matches!(键.as_str(), "contrastBorder" | "contrastActiveBorder")
                            && !registered.iter().any(|(路径, 注册)| {
                                路径 == &规范
                                    && matches!(
                                        注册["uiTheme"].as_str(),
                                        Some("hc-black" | "hc-light")
                                    )
                            }))
                })
                .collect();
            if !缺失.is_empty() {
                println!(
                    "提示 {文件名}：{} 个内置键使用 VS Code 默认颜色",
                    缺失.len()
                );
            }
            for 键 in 颜色表.keys() {
                our_keys.insert(键.clone());
            }
            for (键, 值) in 颜色表 {
                if !是有效颜色(Some(值)) {
                    println!("失败 {文件名}: {键} 必须使用六位或八位十六进制颜色");
                    *failures += 1;
                }
            }
        }
        if let Some(规则表) = theme["tokenColors"].as_array() {
            for 规则 in 规则表 {
                let 有范围 = match &规则["scope"] {
                    Value::String(文本) => !文本.is_empty(),
                    Value::Array(项) => !项.is_empty() && 项.iter().all(Value::is_string),
                    _ => false,
                };
                let 有设置 = 规则["settings"]
                    .as_object()
                    .is_some_and(|设置| !设置.is_empty());
                if !有范围 || !有设置 {
                    println!("失败 {文件名}: 语法规则不完整 {规则}");
                    *failures += 1;
                }
                if let Some(设置) = 规则["settings"].as_object() {
                    for 键 in ["foreground", "background"] {
                        if 设置.contains_key(键) && !是有效颜色(设置.get(键)) {
                            println!("失败 {文件名}: 语法规则 {键} 必须使用六位或八位十六进制颜色");
                            *failures += 1;
                        }
                    }
                }
            }
        }
        if let Some(语义表) = theme["semanticTokenColors"].as_object() {
            for (名称, 值) in 语义表 {
                if !是有效颜色(Some(值)) {
                    println!("失败 {文件名}: 语义标记 {名称} 必须使用六位或八位十六进制颜色");
                    *failures += 1;
                }
            }
        }
    }
    if labels.len() != labels.iter().collect::<HashSet<_>>().len() {
        println!("失败 主题名称重复");
        *failures += 1;
    }

    *failures += 校验产品图标(根目录, &manifest)?;

    // 对每个生成的主题做对比度检查（含强调色、变体、高对比度）。
    let checks: [(&str, &str, f64); 5] = [
        ("editor.foreground", "editor.background", 4.5),
        ("button.foreground", "button.background", 2.5), // GNOME 黄色的对比度约为 2.8
        ("textLink.foreground", "editor.background", 3.0),
        (
            "gitDecoration.deletedResourceForeground",
            "editor.background",
            3.0,
        ),
        ("descriptionForeground", "editor.background", 2.5),
    ];
    for path in &paths {
        let 文本 = std::fs::read_to_string(path)?;
        let theme: Value = serde_json::from_str(&文本)
            .map_err(|错误| 工具错误::新(format!("{} 解析失败：{错误}", path.display())))?;
        let 颜色表 = theme["colors"].as_object().cloned().unwrap_or_default();
        let mut problems: Vec<String> = Vec::new();
        for (前景键, 背景键, 最小) in checks {
            let 前景 = 颜色表.get(前景键);
            let 背景 = 颜色表.get(背景键);
            if !是有效颜色(前景) || !是有效颜色(背景) {
                problems.push(format!("{前景键}/{背景键} 缺少有效颜色"));
                continue;
            }
            let ratio = 对比度(
                前景.and_then(Value::as_str).unwrap_or(""),
                背景.and_then(Value::as_str).unwrap_or(""),
            )?;
            if ratio < 最小 {
                problems.push(format!("{前景键}/{背景键} {ratio:.2} < {最小}"));
            }
        }
        let 状态 = if problems.is_empty() {
            "通过"
        } else {
            "失败"
        };
        println!("{状态} {}", theme["name"].as_str().unwrap_or(""));
        for 问题 in &problems {
            println!("       {问题}");
            *failures += 1;
        }
        if problems.is_empty() {
            通过主题 += 1;
        } else {
            failed_themes += 1;
        }
    }
    println!("对比度：{通过主题} 个主题通过，{failed_themes} 个主题存在问题");

    let 补充 = 补充颜色键();
    let mut allowed: HashSet<&str> = registry
        .iter()
        .chain(builtin.iter())
        .map(String::as_str)
        .collect();
    allowed.extend(补充.iter().copied());
    let mut unknown: Vec<&String> = our_keys
        .iter()
        .filter(|键| !allowed.contains(键.as_str()))
        .collect();
    unknown.sort();
    let mut missing: Vec<&String> = builtin
        .iter()
        .filter(|键| !our_keys.contains(*键))
        .collect();
    missing.sort();
    println!(
        "\n颜色键：已定义 {} 个 | 未覆盖内置键 {} 个 | 未知键 {} 个",
        our_keys.len(),
        missing.len(),
        unknown.len()
    );
    if !unknown.is_empty() {
        println!("未知颜色键（可能存在拼写错误）：");
        for 键 in &unknown {
            println!("  {键}");
        }
        *failures += 1;
    }
    if !missing.is_empty() {
        println!("未覆盖的颜色键（VS Code 将使用默认主题）：");
        for 键 in missing.iter().take(60) {
            println!("  {键}");
        }
        if missing.len() > 60 {
            println!("  ……另有 {} 个", missing.len() - 60);
        }
    }
    println!("\n主题数量：{}", labels.len());
    Ok(*failures)
}

fn 校验产品图标(根目录: &Path, manifest: &Value) -> 结果<i32> {
    let mut failures = 0;
    let 图标路径 = 根目录.join("产品图标").join("adwcode.json");
    let 注册列表 = manifest["contributes"]["productIconThemes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let 已注册 = 注册列表.iter().any(|entry| {
        entry["id"].as_str() == Some("adwcode")
            && entry["path"]
                .as_str()
                .is_some_and(|路径| 规范路径(&根目录.join(路径)) == 规范路径(&图标路径))
    });
    if !已注册 {
        println!("失败：package.json 未正确注册 AdwCode 产品图标主题");
        failures += 1;
    }
    if !图标路径.exists() {
        println!("失败：缺少 产品图标/adwcode.json");
        return Ok(failures + 1);
    }
    let 文本 = std::fs::read_to_string(&图标路径)?;
    let icons: Value = serde_json::from_str(&文本)
        .map_err(|错误| 工具错误::新(format!("产品图标/adwcode.json 解析失败：{错误}")))?;
    let fonts = icons["fonts"].as_array().cloned().unwrap_or_default();
    let definitions = icons["iconDefinitions"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    if fonts.is_empty() || definitions.is_empty() {
        println!("失败 产品图标/adwcode.json: 缺少 fonts 或 iconDefinitions");
        failures += 1;
    }
    for 字体 in &fonts {
        let sources = 字体["src"].as_array().cloned().unwrap_or_default();
        if sources.is_empty() {
            println!(
                "失败 product-icons：字体 {} 缺少来源文件",
                字体["id"].as_str().unwrap_or("")
            );
            failures += 1;
        }
        for 来源 in &sources {
            let 目标 = 来源["path"]
                .as_str()
                .map(|路径| 规范路径(&图标路径.parent().unwrap().join(路径)));
            if !目标.as_deref().is_some_and(|路径| {
                路径.is_file()
                    && std::fs::canonicalize(路径).is_ok_and(|目标| {
                        std::fs::canonicalize(根目录).is_ok_and(|根| 目标.starts_with(根))
                    })
            }) {
                println!(
                    "失败 product-icons：缺少 {}",
                    来源["path"].as_str().unwrap_or("")
                );
                failures += 1;
            }
        }
    }
    let font_ids: Vec<&str> = fonts
        .iter()
        .filter_map(|字体| 字体["id"].as_str())
        .collect();
    if font_ids.len() != fonts.len()
        || font_ids.iter().any(|标识| 标识.is_empty())
        || font_ids.len() != font_ids.iter().collect::<HashSet<_>>().len()
    {
        println!("失败 product-icons: 字体标识重复");
        failures += 1;
    }
    // 同一字形可服务于语义相同的多个产品图标；检查引用，允许有意复用。
    for (icon, definition) in &definitions {
        let character = definition["fontCharacter"].as_str().unwrap_or("");
        if !font_ids.contains(&definition["fontId"].as_str().unwrap_or("")) {
            println!("失败 product-icons: {icon} 引用了未知字体");
            failures += 1;
        }
        if !是字形码点(character) {
            println!("失败 product-icons: {icon} 的字形码点无效");
            failures += 1;
        }
    }
    println!(
        "产品图标：{} 个图标映射，{} 个字体",
        definitions.len(),
        fonts.len()
    );
    Ok(failures)
}

/// `adwcode 主题` 的入口。
pub fn 生成入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let mut watch = false;
    for 参数项 in 参数 {
        match 参数项.as_str() {
            "--监视" => watch = true,
            其他 => {
                return Err(工具错误::新(format!("未知参数：{其他}")));
            }
        }
    }
    let entries = 写入主题(根目录, &构建计划(&["blue"]), watch)?;
    if watch {
        println!("监视模式：已为主题条目添加 _watch 标记");
    }

    for 条目 in &entries {
        println!("  {:9} {}", 条目.ui_theme, 条目.label);
    }
    println!("已将 {} 个主题写入 主题/", entries.len());
    Ok(())
}

/// `adwcode 校验` 的入口。
pub fn 校验入口(根目录: &Path) -> 结果<()> {
    let failures = 校验(根目录)?;
    if failures == 0 {
        Ok(())
    } else {
        Err(工具错误::新(format!("校验失败：{failures} 项")))
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 根目录() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn 完整仓库的语法字段缺失不能被其他主题掩盖() {
        fn 复制(来源: &Path, 目标: &Path) {
            std::fs::create_dir_all(目标).unwrap();
            for 条目 in std::fs::read_dir(来源).unwrap() {
                let 条目 = 条目.unwrap();
                let 路径 = 目标.join(条目.file_name());
                if 条目.file_type().unwrap().is_dir() {
                    复制(&条目.path(), &路径);
                } else {
                    std::fs::copy(条目.path(), 路径).unwrap();
                }
            }
        }
        let 临时 = tempfile::tempdir().unwrap();
        for 名称 in ["主题", "产品图标", "源码/VSCode默认数据"] {
            复制(&根目录().join(名称), &临时.path().join(名称));
        }
        std::fs::copy(
            根目录().join("package.json"),
            临时.path().join("package.json"),
        )
        .unwrap();
        assert_eq!(校验(临时.path()).unwrap(), 0);
        let 路径 = 临时.path().join("主题/adwcode-深色.json");
        let 原文 = std::fs::read_to_string(&路径).unwrap();
        for 字段 in ["tokenColors", "semanticTokenColors"] {
            let mut 主题: Value = serde_json::from_str(&原文).unwrap();
            主题.as_object_mut().unwrap().remove(字段);
            std::fs::write(&路径, 写json(&主题)).unwrap();
            assert!(校验(临时.path()).unwrap() > 0, "{字段}");
        }
        assert!(!是字形码点("\\d800"));
        assert!(是字形码点("\\e300"));
    }

    #[test]
    fn 主题与仓库产物字节一致() {
        let plan = 构建计划(&["blue"]);
        for 请求 in &plan {
            let 文件名 = 主题文件名(请求.mode, 请求.accent, 请求.variant, 请求.hc);
            let 路径 = 根目录().join("主题").join(&文件名);
            let 期望 = std::fs::read_to_string(&路径).expect("读取已提交主题");
            let 生成 = 写json(&生成主题对象(根目录(), 请求).expect("生成主题"));
            assert_eq!(生成, 期望, "{文件名} 与提交产物不一致");
        }
    }

    #[test]
    fn 校验通过且清单序列化稳定() {
        assert_eq!(校验(根目录()).expect("执行校验"), 0);
        let 文本 = std::fs::read_to_string(根目录().join("package.json")).expect("读取清单");
        let entries = 注册项(根目录(), &构建计划(&["blue"]), false).expect("生成注册项");
        let mut manifest: Value = serde_json::from_str(&文本).expect("解析清单");
        let 列表 = entries
            .iter()
            .map(|条目| {
                let mut 对象 = Map::new();
                对象.insert("label".to_string(), Value::String(条目.label.clone()));
                对象.insert("uiTheme".to_string(), Value::String(条目.ui_theme.clone()));
                对象.insert("path".to_string(), Value::String(条目.path.clone()));
                Value::Object(对象)
            })
            .collect();
        manifest["contributes"]["themes"] = Value::Array(列表);
        assert_eq!(
            写json(&manifest),
            文本,
            "package.json 序列化与提交内容不一致"
        );
    }

    /// 最小校验仓库：键表、清单、主题与产品图标都可按用例改写。
    struct 校验仓库 {
        目录: PathBuf,
    }

    impl 校验仓库 {
        fn 新建(名称: &str) -> Self {
            let 目录 =
                std::env::temp_dir().join(format!("adwcode-校验-{名称}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&目录);
            std::fs::create_dir_all(目录.join("主题")).expect("创建主题目录");
            std::fs::create_dir_all(目录.join("源码/VSCode默认数据")).expect("创建数据目录");
            std::fs::write(目录.join("源码/VSCode默认数据/builtin_keys.json"), "[]")
                .expect("写入内置键");
            std::fs::write(目录.join("源码/VSCode默认数据/registry_keys.json"), "[]")
                .expect("写入注册表键");
            Self { 目录 }
        }

        fn 写清单(&self, 内容: &str) {
            std::fs::write(self.目录.join("package.json"), 内容).expect("写入清单");
        }

        fn 写主题(&self, 名字: &str, 内容: &str) {
            std::fs::write(self.目录.join("主题").join(名字), 内容).expect("写入主题");
        }
    }

    impl Drop for 校验仓库 {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.目录);
        }
    }

    fn 有效主题(colors: &str) -> String {
        format!(
            r#"{{"$schema":"vscode://schemas/color-theme","name":"AdwCode 深色","type":"dark","semanticHighlighting":true,"colors":{colors},"tokenColors":[],"semanticTokenColors":{{}}}}"#
        )
    }

    #[test]
    fn 缺少键表时校验失败() {
        let 仓库 = 校验仓库::新建("缺键表");
        std::fs::remove_file(仓库.目录.join("源码/VSCode默认数据/builtin_keys.json")).unwrap();
        仓库.写主题("adwcode-深色.json", &有效主题("{}"));
        仓库.写清单(r#"{"contributes":{"themes":[]}}"#);
        assert_eq!(校验(&仓库.目录).expect("执行校验"), 1);
    }

    #[test]
    fn 未注册与非法颜色被拒绝() {
        let 仓库 = 校验仓库::新建("非法");
        仓库.写主题(
            "adwcode-深色.json",
            &有效主题(r#"{"editor.background":"rgb(0 0 0)"}"#),
        );
        // 主题文件存在但未在清单注册，同时颜色不是十六进制。
        仓库.写清单(
            r#"{"contributes":{"themes":[],"productIconThemes":[{"id":"adwcode","path":"./产品图标/adwcode.json"}]}}"#,
        );
        仓库.写主题("adwcode-浅色.json", &有效主题("{}"));
        let 失败 = 校验(&仓库.目录).expect("执行校验");
        assert!(失败 >= 2, "应同时报告未注册与非法颜色，实际 {失败}");
    }

    #[test]
    fn 产品图标缺失来源被拒绝() {
        let 仓库 = 校验仓库::新建("图标");
        仓库.写主题("adwcode-深色.json", &有效主题("{}"));
        let 图标目录 = 仓库.目录.join("产品图标");
        std::fs::create_dir_all(&图标目录).unwrap();
        std::fs::write(
            图标目录.join("adwcode.json"),
            r#"{"fonts":[{"id":"adwcode","src":[{"path":"缺失.ttf"}]}],"iconDefinitions":{"adwcode.icon":{"fontId":"adwcode","fontCharacter":"\\e001"}}}"#,
        )
        .unwrap();
        仓库.写清单(
            r#"{"contributes":{"themes":[{"label":"AdwCode 深色","uiTheme":"vs-dark","path":"./主题/adwcode-深色.json"}],"productIconThemes":[{"id":"adwcode","path":"./产品图标/adwcode.json"}]}}"#,
        );
        let 失败 = 校验(&仓库.目录).expect("执行校验");
        assert!(失败 >= 1, "应报告缺失字体来源，实际 {失败}");
    }
    #[test]
    fn 对比度按透明前景合成() {
        assert_eq!(对比度("#00000000", "#ffffff").unwrap(), 1.0);
        assert!(对比度("#00000080", "#ffffff").unwrap() < 对比度("#000000", "#ffffff").unwrap());
        assert!(对比度("#000", "#fff0").is_err());
    }
    #[test]
    fn 多光标在编辑器背景上可辨识() {
        for 请求 in 构建计划(&["blue"]) {
            let 主题 = 生成主题对象(根目录(), &请求).unwrap();
            let 颜色 = &主题["colors"];
            for 键 in [
                "editorMultiCursor.primary.foreground",
                "editorMultiCursor.secondary.foreground",
            ] {
                assert!(
                    对比度(
                        颜色[键].as_str().unwrap(),
                        颜色["editor.background"].as_str().unwrap()
                    )
                    .unwrap()
                        >= 3.0,
                    "{} 的 {键} 必须可辨识",
                    请求.mode
                );
            }
        }
    }
}
