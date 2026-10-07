// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 对照已安装的 VS Code 检查 `附加外观/` 中的自定义 CSS。
//!
//! VS Code 更名类名或移除设计令牌时，CSS 补丁就会失效。本模块解析项目样式表并校验：
//!
//! - 原生类选择器仍存在于 VS Code 编译后的 CSS 或 JavaScript 中；
//!   项目状态类由附加脚本的显式 `classList` 创建操作定义；
//! - 样式引用的每个 `var(--vscode-*)` 要么由 VS Code 定义、属于主题色注册表，
//!   要么由项目的令牌块定义。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::错误::{工具错误, 结果};

static 注释正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)/\*.*?\*/").expect("注释正则"));
static 块正则: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{[^{}]*\}").expect("块正则"));
static 类名正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.([a-zA-Z][\w-]*)").expect("类名正则"));
static 变量引用正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var\(\s*(--vscode-[\w-]+)").expect("变量引用正则"));
static 变量定义正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(--vscode-[\w-]+)\s*:").expect("变量定义正则"));
static 脚本类名正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"["'`]([a-zA-Z][\w-]*)["'`]"#).expect("脚本类名正则"));
static 创建类名正则: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"classList\.(?:add|toggle)\(\s*["']([\w-]+)["']"#).expect("创建类名正则")
});

/// 返回已安装 VS Code 的（workbench CSS, workbench JS）。
#[must_use]
pub fn 查找vscode资源(显式: Option<&Path>) -> (Option<PathBuf>, Option<PathBuf>) {
    if let Some(路径) = 显式 {
        if 路径.is_file() {
            let js = 路径.with_extension("js");
            let js = js.is_file().then_some(js);
            return (Some(路径.to_path_buf()), js);
        }
        return (None, None);
    }
    let mut 候选: Vec<PathBuf> = vec![
        PathBuf::from("/home/linuxbrew/.linuxbrew/Caskroom/visual-studio-code-linux"),
        PathBuf::from("/usr/share/code/resources/app/out/vs/workbench"),
        PathBuf::from("/usr/lib/code/resources/app/out/vs/workbench"),
        PathBuf::from("/opt/visual-studio-code/resources/app/out/vs/workbench"),
        PathBuf::from("/usr/share/code-oss/resources/app/out/vs/workbench"),
    ];
    if let Some(主目录) = std::env::var_os("HOME") {
        候选.push(PathBuf::from(主目录).join(".local/share/code/resources/app/out/vs/workbench"));
    }
    for 项目 in 候选 {
        let mut 匹配: Vec<PathBuf> = Vec::new();
        if 项目.is_dir() {
            递归查找(&项目, "workbench.desktop.main.css", &mut 匹配);
            匹配.sort();
        }
        if let Some(css) = 匹配.into_iter().next() {
            let js = css.with_extension("js");
            let js = js.is_file().then_some(js);
            return (Some(css), js);
        }
    }
    (None, None)
}

fn 递归查找(目录: &Path, 文件名: &str, 输出: &mut Vec<PathBuf>) {
    let Ok(条目列表) = std::fs::read_dir(目录) else {
        return;
    };
    for 条目 in 条目列表.flatten() {
        let 路径 = 条目.path();
        if 路径.is_dir() {
            递归查找(&路径, 文件名, 输出);
        } else if 路径.file_name().is_some_and(|名字| 名字 == 文件名) {
            输出.push(路径);
        }
    }
}

/// 主题色注册表的键会被 VS Code 以 `--vscode-<键，点换横线>` 注入。
pub fn 主题变量(注册表路径: &Path) -> 结果<HashSet<String>> {
    if !注册表路径.is_file() {
        return Ok(HashSet::new());
    }
    let 文本 = std::fs::read_to_string(注册表路径)?;
    let 键表: Vec<String> = serde_json::from_str(&文本)
        .map_err(|错误| 工具错误::新(format!("registry_keys.json 解析失败：{错误}")))?;
    Ok(键表
        .into_iter()
        .map(|键| format!("--vscode-{}", 键.replace('.', "-")))
        .collect())
}

/// 把样式表拆分为选择器文本与声明文本。
#[must_use]
pub fn 选择器与声明(文本: &str) -> (String, String) {
    let 无注释 = 注释正则.replace_all(文本, "");
    let 选择器 = 块正则.replace_all(&无注释, " ").to_string();
    let 声明 = 块正则
        .find_iter(&无注释)
        .map(|匹配| {
            let 块 = 匹配.as_str();
            &块[1..块.len() - 1]
        })
        .collect::<Vec<_>>()
        .join(" ");
    (选择器, 声明)
}

fn 读取类名(文本: &str) -> HashSet<String> {
    类名正则
        .captures_iter(文本)
        .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string()))
        .collect()
}

/// 执行校验；返回失败项数量。
pub fn 校验(根目录: &Path, 显式: Option<&Path>, 详细: bool) -> 结果<i32> {
    let (vscode_css, vscode_js) = 查找vscode资源(显式);
    let Some(vscode_css) = vscode_css else {
        println!("未执行 CSS 校验：未找到 VS Code 样式表，请使用 --样式表 PATH 指定有效路径");
        return Ok(1);
    };
    let vscode_text = String::from_utf8_lossy(&std::fs::read(&vscode_css)?).into_owned();
    let mut vscode_classes = 读取类名(&选择器与声明(&vscode_text).0);
    let mut vscode_vars: HashSet<String> = 变量定义正则
        .captures_iter(&vscode_text)
        .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string()))
        .collect();
    vscode_vars.extend(主题变量(
        &根目录.join("源码/VSCode默认数据/registry_keys.json"),
    )?);
    // 由 JavaScript 创建的类名（例如窗口控制按钮）不会出现在编译后的 CSS 里，
    // 因此同时搜索 JS bundle。
    if let Some(js) = vscode_js {
        let js_text = String::from_utf8_lossy(&std::fs::read(&js)?).into_owned();
        vscode_classes.extend(读取类名(&js_text));
        vscode_classes.extend(
            脚本类名正则
                .captures_iter(&js_text)
                .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string())),
        );
    }

    // 附加脚本创建的状态类不属于 VS Code；只认可显式的 classList 创建操作。
    let 附加目录 = 根目录.join("附加外观");
    let mut project_classes: HashSet<String> = HashSet::new();
    let mut 附加脚本: Vec<PathBuf> = std::fs::read_dir(&附加目录)?
        .flatten()
        .map(|条目| 条目.path())
        .filter(|路径| 路径.extension().is_some_and(|扩展| 扩展 == "ts"))
        .collect();
    附加脚本.sort();
    for script in 附加脚本 {
        let 文本 = std::fs::read_to_string(&script)?;
        project_classes.extend(
            创建类名正则
                .captures_iter(&文本)
                .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string())),
        );
    }
    let mut 失败 = 0;
    let mut 样式列表: Vec<PathBuf> = std::fs::read_dir(&附加目录)?
        .flatten()
        .map(|条目| 条目.path())
        .filter(|路径| 路径.extension().is_some_and(|扩展| 扩展 == "css"))
        .collect();
    样式列表.sort();
    for sheet in 样式列表 {
        let 文本 = std::fs::read_to_string(&sheet)?;
        let (选择器, 声明) = 选择器与声明(&文本);
        let 类名 = 读取类名(&选择器);
        let mut 缺失: Vec<&String> = 类名
            .iter()
            .filter(|名称| !vscode_classes.contains(*名称) && !project_classes.contains(*名称))
            .collect();
        缺失.sort();
        let 文件名 = sheet
            .file_name()
            .map_or_else(String::new, |名字| 名字.to_string_lossy().to_string());
        if 缺失.is_empty() {
            println!(
                "通过 {文件名}：{} 个类名由 VS Code 或项目脚本定义",
                类名.len()
            );
        } else {
            失败 += 1;
            println!(
                "失败 {文件名}：{} 个类名未出现在 {} 中",
                缺失.len(),
                vscode_css
                    .file_name()
                    .map_or_else(String::new, |名字| 名字.to_string_lossy().to_string())
            );
            for 名称 in &缺失 {
                println!("       .{名称}");
            }
        }

        let 本地变量: HashSet<String> = 变量定义正则
            .captures_iter(&文本)
            .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string()))
            .collect();
        let 引用变量: HashSet<String> = 变量引用正则
            .captures_iter(&声明)
            .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string()))
            .collect();
        let mut 未定义: Vec<&String> = 引用变量
            .iter()
            .filter(|名称| !vscode_vars.contains(*名称) && !本地变量.contains(*名称))
            .collect();
        未定义.sort();
        if 未定义.is_empty() {
            if 详细 {
                println!("     {文件名}: {} 个变量均已定义", 引用变量.len());
            }
        } else {
            失败 += 1;
            println!("失败 {文件名}：{} 个变量未定义", 未定义.len());
            for 名称 in &未定义 {
                println!("       {名称}");
            }
        }
    }
    Ok(失败)
}

/// `adwcode 检查样式` 的入口。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let mut 样式表: Option<PathBuf> = None;
    let mut 详细 = false;
    let mut 序号 = 0;
    while 序号 < 参数.len() {
        match 参数[序号].as_str() {
            "--样式表" => {
                序号 += 1;
                样式表 = Some(PathBuf::from(
                    参数
                        .get(序号)
                        .ok_or_else(|| 工具错误::新("--样式表 缺少取值"))?,
                ));
            }
            "--详细" => 详细 = true,
            其他 => return Err(工具错误::新(format!("未知参数：{其他}"))),
        }
        序号 += 1;
    }
    let 失败 = 校验(根目录, 样式表.as_deref(), 详细)?;
    if 失败 == 0 {
        Ok(())
    } else {
        Err(工具错误::新(format!("CSS 校验失败：{失败} 个样式表")))
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 附加类名必须由ts源码显式创建() {
        let 临时 = tempfile::tempdir().unwrap();
        let 根 = 临时.path();
        std::fs::create_dir_all(根.join("源码/VSCode默认数据")).unwrap();
        std::fs::write(根.join("源码/VSCode默认数据/registry_keys.json"), "[]").unwrap();
        std::fs::create_dir(根.join("附加外观")).unwrap();
        std::fs::write(
            根.join("附加外观/样式.css"),
            ".adwcode-window-inactive { color: red; }",
        )
        .unwrap();
        let 脚本 = 根.join("附加外观/窗口状态.ts");
        std::fs::write(
            &脚本,
            "part.classList.toggle('adwcode-window-inactive', inactive);",
        )
        .unwrap();
        let 工作台 = 根.join("workbench.css");
        std::fs::write(&工作台, "").unwrap();
        assert_eq!(校验(根, Some(&工作台), false).unwrap(), 0);
        std::fs::write(&脚本, "part.classList.remove('adwcode-window-inactive');").unwrap();
        assert_eq!(校验(根, Some(&工作台), false).unwrap(), 1);
    }

    #[test]
    fn 拆分选择器与声明() {
        let 文本 = "/* 注释 */ .one, .two { color: red; }\n.three{ padding: 0 }";
        let (选择器, 声明) = 选择器与声明(文本);
        assert!(选择器.contains(".one"));
        assert!(选择器.contains(".three"));
        assert!(!选择器.contains("color"));
        assert!(声明.contains("color: red;"));
        assert!(声明.contains("padding: 0"));

        let 类名 = 读取类名(&选择器);
        assert!(类名.contains("one"));
        assert!(类名.contains("two"));
        assert!(类名.contains("three"));
    }

    #[test]
    fn 识别变量与创建类名() {
        let 文本 = "var(--vscode-editor-background) var(--vscode-adwcode-x)";
        let 引用: HashSet<String> = 变量引用正则
            .captures_iter(文本)
            .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string()))
            .collect();
        assert!(引用.contains("--vscode-editor-background"));
        assert!(引用.contains("--vscode-adwcode-x"));

        let 脚本 =
            r#"element.classList.add("window-max-restore"); element.classList.toggle('active');"#;
        let 创建: Vec<String> = 创建类名正则
            .captures_iter(脚本)
            .filter_map(|捕获| 捕获.get(1).map(|匹配| 匹配.as_str().to_string()))
            .collect();
        assert_eq!(创建, vec!["window-max-restore", "active"]);
    }
}
