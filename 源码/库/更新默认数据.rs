// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 从 microsoft/vscode 刷新颜色键表。
//!
//! `builtin_keys.json` 保存内置主题颜色键并集，`registry_keys.json` 保存官方颜色
//! 标识符表，用于主题映射校验。下载与解析全部成功后再写入文件。

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;

use crate::错误::{工具错误, 结果};

const 主题源: &str =
    "https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes";
const 文档源: &str =
    "https://raw.githubusercontent.com/microsoft/vscode-docs/main/api/references/theme-color.md";

/// 参与并集的 VS Code 内置主题。
pub const 全部主题: [&str; 10] = [
    "2026-dark",
    "2026-light",
    "dark_modern",
    "dark_plus",
    "dark_vs",
    "light_modern",
    "light_plus",
    "light_vs",
    "hc_black",
    "hc_light",
];

static 带点键正则: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`([a-zA-Z][a-zA-Z0-9]*(?:\.[a-zA-Z0-9]+)+)`").expect("带点键正则")
});
static 单键正则: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^- `([a-zA-Z][a-zA-Z0-9]*)`:").expect("单键正则"));

/// 默认主题必须是 JSON 对象；只扩展注释与尾逗号，拒绝 JSON5 的其他语法。
fn 解析jsonc(文本: &str) -> 结果<serde_json::Value> {
    let 选项 = jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    };
    let 值: serde_json::Value = jsonc_parser::parse_to_serde_value(文本, &选项)
        .map_err(|错误| 工具错误::新(format!("默认主题 JSONC 解析失败：{错误}")))?;
    if !值.is_object() {
        return Err(工具错误::新("默认主题 JSONC 根节点必须是对象"));
    }
    Ok(值)
}

fn 创建代理(超时秒: u64) -> 结果<ureq::Agent> {
    let 配置 = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(超时秒)))
        .build();
    Ok(配置.new_agent())
}

fn 获取(代理: &ureq::Agent, 地址: &str) -> 结果<String> {
    let mut 响应 = 代理
        .get(地址)
        .call()
        .map_err(|错误| 工具错误::新(format!("下载失败 {地址}：{错误}")))?;
    响应
        .body_mut()
        .read_to_string()
        .map_err(|错误| 工具错误::新(format!("读取响应失败 {地址}：{错误}")))
}

/// `adwcode 更新默认数据` 的入口。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    if !参数.is_empty() {
        return Err(工具错误::新(format!("未知参数：{}", 参数[0])));
    }
    let 主题代理 = 创建代理(30)?;
    let 文档代理 = 创建代理(60)?;
    更新(根目录, &|地址, 超时秒| {
        if 超时秒 <= 30 {
            获取(&主题代理, 地址)
        } else {
            获取(&文档代理, 地址)
        }
    })
}

/// 更新逻辑主体；下载器可注入以便离线测试。
fn 更新(根目录: &Path, 下载: &dyn Fn(&str, u64) -> 结果<String>) -> 结果<()> {
    let 输出目录 = 根目录.join("源码/VSCode默认数据");
    // 所有下载和解析成功后再写文件，避免中途失败留下混合版本的数据。
    let mut 键: BTreeSet<String> = BTreeSet::new();
    for 名称 in 全部主题 {
        let 文本 = 下载(&format!("{主题源}/{名称}.json"), 30)?;
        let 值: serde_json::Value = 解析jsonc(&文本)
            .map_err(|错误| 工具错误::新(format!("{名称}.json 解析失败：{错误}")))?;
        let 颜色 = 值
            .get("colors")
            .and_then(serde_json::Value::as_object)
            .filter(|颜色| !颜色.is_empty())
            .ok_or_else(|| 工具错误::新(format!("{名称}.json 缺少非空 colors 对象")))?;
        键.extend(颜色.keys().cloned());
    }
    let 内置文本 = 写字符串数组(&键);
    let mut 消息 = vec![format!("内置颜色键： {}", 键.len())];

    let markdown = 下载(文档源, 60)?;
    let mut 注册表: BTreeSet<String> = BTreeSet::new();
    for 捕获 in 带点键正则.captures_iter(&markdown) {
        if let Some(匹配) = 捕获.get(1) {
            注册表.insert(匹配.as_str().to_string());
        }
    }
    for 捕获 in 单键正则.captures_iter(&markdown) {
        if let Some(匹配) = 捕获.get(1) {
            注册表.insert(匹配.as_str().to_string());
        }
    }
    注册表.retain(|键| {
        !键.starts_with("workbench.")
            && !键.starts_with("editor.token")
            && !键.starts_with("configuration.")
            && !键.starts_with("vscode.")
    });
    if 注册表.is_empty() {
        return Err(工具错误::新(
            "主题颜色文档未解析到任何注册键，保留原数据",
        ));
    }
    let 注册表文本 = 写字符串数组(&注册表);
    消息.push(format!("注册表颜色键： {}", 注册表.len()));
    crate::文件事务::写入批次(&[
        (
            输出目录.join("builtin_keys.json"),
            Some(内置文本.into_bytes()),
        ),
        (
            输出目录.join("registry_keys.json"),
            Some(注册表文本.into_bytes()),
        ),
    ])?;
    for 行 in 消息 {
        println!("{行}");
    }
    Ok(())
}

fn 写字符串数组(值: &BTreeSet<String>) -> String {
    let 列表: Vec<serde_json::Value> = 值
        .iter()
        .map(|项| serde_json::Value::String(项.clone()))
        .collect();
    crate::主题生成::写json(&serde_json::Value::Array(列表))
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 只接受注释与尾逗号且保留字符串() {
        let 文本 = r#"{
          // 行注释
          "符号": "值,} // /*",
          "转义": "引号\"与反斜杠\\",
          /* 块
          注释 */
          "a": "旧值", "b": [1, 2,],
          "a": "https://example.invalid/值,}",
        }"#;
        let 值 = 解析jsonc(文本).unwrap();
        assert_eq!(值["a"], "https://example.invalid/值,}");
        assert_eq!(值["b"], serde_json::json!([1, 2]));
        assert_eq!(值["符号"], "值,} // /*");
        assert_eq!(值["转义"], "引号\"与反斜杠\\");
        for 文本 in [
            r#"{"a":1/*注释*/2}"#,
            r#"{"a":1}/*未闭合"#,
            r#"{a:1}"#,
            r#"{'a':1}"#,
            r#"{"a":1 "b":2}"#,
            r#"{"a":[1 2]}"#,
            r#"{"a":0x10}"#,
            r#"{"a":+1}"#,
            r#"{"a":.5}"#,
            r#"{"a":5.}"#,
            r#"{"a":NaN}"#,
            r#"{"a":Infinity}"#,
            r#"{"a":"\x41"}"#,
            r#"{"a":"\v"}"#,
            r#"{"a":01}"#,
            r#"{"a":1e400}"#,
            "",
            "// 只有注释",
            "[]",
            "null",
        ] {
            assert!(解析jsonc(文本).is_err(), "{文本}");
        }
        let 错误 = 解析jsonc("{\n/* 行二\n行三 */\n\"a\":1 2}").unwrap_err();
        assert!(错误.消息.contains("line 4"), "{}", 错误.消息);
    }

    #[test]
    fn 已提交键表序列化稳定() {
        let 根 = Path::new(env!("CARGO_MANIFEST_DIR"));
        for 名字 in ["builtin_keys.json", "registry_keys.json"] {
            let 路径 = 根.join("源码/VSCode默认数据").join(名字);
            let 文本 = std::fs::read_to_string(&路径).expect("读取键表");
            let 列表: Vec<String> = serde_json::from_str(&文本).expect("解析键表");
            let 集合: BTreeSet<String> = 列表.into_iter().collect();
            assert_eq!(写字符串数组(&集合), 文本, "{名字} 序列化不一致");
        }
    }

    #[test]
    fn 下载失败保留既有数据且成功时正确并集() {
        let 根 = std::env::temp_dir().join(format!("adwcode-默认数据-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(根.join("源码/VSCode默认数据")).expect("创建数据目录");
        std::fs::write(
            根.join("源码/VSCode默认数据/builtin_keys.json"),
            "[\"旧\"]\n",
        )
        .expect("写入旧数据");
        std::fs::write(
            根.join("源码/VSCode默认数据/registry_keys.json"),
            "[\"旧\"]\n",
        )
        .expect("写入旧数据");

        // 失败：最后一个主题下载报错，既有文件保持原样。
        let 失败下载 = |地址: &str, _: u64| -> 结果<String> {
            if 地址.ends_with("hc_light.json") {
                Err(工具错误::新("网络失败"))
            } else {
                Ok("{\"colors\":{\"a.b\":\"#000\"}}".to_string())
            }
        };
        assert!(更新(&根, &失败下载).is_err());
        assert_eq!(
            std::fs::read_to_string(根.join("源码/VSCode默认数据/builtin_keys.json")).unwrap(),
            "[\"旧\"]\n"
        );

        // 成功：并集、JSONC 清理、正则提取与前缀过滤。
        let 成功下载 = |地址: &str, _: u64| -> 结果<String> {
            if 地址 == 文档源 {
                Ok("`editor.background`\n`workbench.colorCustomizations`\n- `foreground`:\n- `editorBracketMatch.background`:\n`vscode.x`\n`configuration.y`\n`editor.tokenColorCustomizations`\n"
                    .to_string())
            } else if 地址.ends_with("dark_vs.json") {
                Ok(
                    "{\n // 注释\n \"colors\": {\"a.b\": \"#000\", \"c\": \"#111\",},\n}"
                        .to_string(),
                )
            } else {
                Ok("{\"colors\": {\"a.b\": \"#000\", \"d\": \"#222\"}}".to_string())
            }
        };
        更新(&根, &成功下载).expect("更新成功");
        let 内置: Vec<String> = serde_json::from_str(
            &std::fs::read_to_string(根.join("源码/VSCode默认数据/builtin_keys.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(内置, vec!["a.b", "c", "d"]);
        let 注册表: Vec<String> = serde_json::from_str(
            &std::fs::read_to_string(根.join("源码/VSCode默认数据/registry_keys.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            注册表,
            vec![
                "editor.background",
                "editorBracketMatch.background",
                "foreground"
            ]
        );
        let _ = std::fs::remove_dir_all(&根);
    }
    #[test]
    fn 空主题和空文档不覆盖数据() {
        let 目录 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(目录.path().join("源码/VSCode默认数据")).unwrap();
        let 路径 = 目录.path().join("源码/VSCode默认数据/builtin_keys.json");
        std::fs::write(&路径, "旧数据").unwrap();
        for 空主题 in [true, false] {
            let 下载 = |地址: &str, _: u64| {
                Ok(if 地址 == 文档源 {
                    "没有颜色键"
                } else if 空主题 {
                    "{}"
                } else {
                    r##"{"colors":{"editor.background":"#fff"}}"##
                }
                .to_string())
            };
            assert!(更新(目录.path(), &下载).is_err());
            assert_eq!(std::fs::read_to_string(&路径).unwrap(), "旧数据");
        }
    }
}
