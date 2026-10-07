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

/// 移除 JSONC 注释，并在字符串之外移除尾逗号。
#[must_use]
pub fn 清理json注释(文本: &str) -> String {
    let 字符: Vec<char> = 文本.chars().collect();
    let 长度 = 字符.len();
    let mut 输出: Vec<char> = Vec::new();
    let (mut 位置, mut 在字符串, mut 转义) = (0, false, false);
    while 位置 < 长度 {
        let 当前 = 字符[位置];
        if 在字符串 {
            输出.push(当前);
            if 转义 {
                转义 = false;
            } else if 当前 == '\\' {
                转义 = true;
            } else if 当前 == '"' {
                在字符串 = false;
            }
            位置 += 1;
            continue;
        }
        if 当前 == '"' {
            在字符串 = true;
            输出.push(当前);
            位置 += 1;
            continue;
        }
        if 当前 == '/' && 位置 + 1 < 长度 && 字符[位置 + 1] == '/' {
            位置 = 字符[位置..]
                .iter()
                .position(|字符| *字符 == '\n')
                .map_or(长度, |偏移| 位置 + 偏移);
            continue;
        }
        if 当前 == '/' && 位置 + 1 < 长度 && 字符[位置 + 1] == '*' {
            let Some(偏移) = 字符[位置..].windows(2).position(|窗口| 窗口 == ['*', '/'])
            else {
                // 保留非法输入，由 JSON 解析器报告未闭合注释。
                return 文本.to_string();
            };
            let 结束 = 位置 + 偏移 + 2;
            输出.extend(
                字符[位置..结束]
                    .iter()
                    .map(|字符| if *字符 == '\n' { '\n' } else { ' ' }),
            );
            位置 = 结束;
            continue;
        }
        输出.push(当前);
        位置 += 1;
    }
    // 仅在字符串之外移除尾逗号，不能改写诸如 ",}" 的字符串值。
    let mut 结果: Vec<char> = Vec::new();
    let (mut 在字符串, mut 转义) = (false, false);
    for (序号, 当前) in 输出.iter().enumerate() {
        if 在字符串 {
            结果.push(*当前);
            if 转义 {
                转义 = false;
            } else if *当前 == '\\' {
                转义 = true;
            } else if *当前 == '"' {
                在字符串 = false;
            }
            continue;
        }
        if *当前 == '"' {
            在字符串 = true;
        }
        if *当前 == ',' {
            let 后续 = 输出[序号 + 1..]
                .iter()
                .copied()
                .find(|字符| !matches!(字符, ' ' | '\t' | '\n' | '\r'));
            if matches!(后续, Some('}' | ']')) {
                continue;
            }
        }
        结果.push(*当前);
    }
    结果.into_iter().collect()
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
        let 值: serde_json::Value = serde_json::from_str(&清理json注释(&文本))
            .map_err(|错误| 工具错误::新(format!("{名称}.json 解析失败：{错误}")))?;
        if let Some(颜色) = 值.get("colors").and_then(serde_json::Value::as_object) {
            键.extend(颜色.keys().cloned());
        }
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
    fn 注释不能连接数字或吞掉未闭合输入() {
        for 文本 in ["{\"a\":1/*注释*/2}", "{\"a\":1}/*未闭合"] {
            assert!(serde_json::from_str::<serde_json::Value>(&清理json注释(文本)).is_err());
        }
        assert_eq!(清理json注释("[1,/*行一\n行二*/2]").matches('\n').count(), 1);
    }

    #[test]
    fn 清理注释与尾逗号() {
        let 文本 = r#"{
  // 行注释
  "a": "值,}", /* 块
  注释 */
  "b": [1, 2,],
}"#;
        let 清理 = 清理json注释(文本);
        assert!(!清理.contains("行注释"));
        assert!(!清理.contains("块"));
        assert!(清理.contains("\"值,}\""));
        let 值: serde_json::Value = serde_json::from_str(&清理).expect("解析清理后的 JSON");
        assert_eq!(值["a"], "值,}");
        assert_eq!(值["b"].as_array().map(Vec::len), Some(2));
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
                Ok("{\"colors\":{}}".to_string())
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
}
