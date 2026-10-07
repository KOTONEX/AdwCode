// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 校验声明式默认设置、命令可用性和工作区能力，不加载扩展 JavaScript。

use crate::错误::{工具错误, 结果};
use serde_json::{Value, json};
use std::path::Path;

const 命令: [&str; 5] = [
    "adwcode.查看外观安装状态",
    "adwcode.安装GNOME外观",
    "adwcode.安装仅关闭窗口控件",
    "adwcode.选择外观组件",
    "adwcode.移除外观引用",
];

fn 要求(成立: bool, 说明: &str) -> 结果<()> {
    if 成立 {
        Ok(())
    } else {
        Err(工具错误::新(format!("清单验证失败：{说明}")))
    }
}

fn 记录列表<'a>(数据: &'a Value, 路径: &str) -> 结果<&'a [Value]> {
    数据
        .pointer(路径)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| 工具错误::新(format!("清单字段必须是数组：{路径}")))
}

/// 校验清单数据；字段缺失、类型错误和重复命令均失败。
pub fn 校验清单(清单: &Value) -> 结果<()> {
    let 贡献 = &清单["contributes"];
    let 默认 = &贡献["configurationDefaults"];
    let 默认 = 默认
        .as_object()
        .ok_or_else(|| 工具错误::新("清单默认设置必须是对象"))?;
    let 主题 = 记录列表(贡献, "/themes")?;
    for 键 in [
        "workbench.preferredLightColorTheme",
        "workbench.preferredDarkColorTheme",
        "workbench.preferredHighContrastLightColorTheme",
        "workbench.preferredHighContrastColorTheme",
    ] {
        let 值 = 默认.get(键).and_then(Value::as_str);
        要求(
            值.is_some_and(|值| 主题.iter().any(|主题| 主题["label"].as_str() == Some(值))),
            键,
        )?;
    }
    let 图标 = 记录列表(贡献, "/productIconThemes")?;
    let 默认图标 = 默认
        .get("workbench.productIconTheme")
        .and_then(Value::as_str);
    要求(
        默认图标.is_some_and(|值| 图标.iter().any(|图标| 图标["id"].as_str() == Some(值))),
        "默认产品图标必须已注册",
    )?;
    要求(
        默认.get("window.menuBarVisibility") == Some(&json!("compact")),
        "菜单默认值应为 compact",
    )?;
    要求(
        默认.get("editor.fontFamily") == Some(&json!("Adwaita Mono, monospace")),
        "编辑器字体默认值",
    )?;
    要求(
        !默认.contains_key("workbench.iconTheme"),
        "不能用默认值清空用户图标主题",
    )?;
    要求(
        贡献.pointer("/configuration/properties/adwcode.界面字体/default") == Some(&json!("")),
        "界面字体默认值应为空串",
    )?;

    let mut 预期 = 命令.to_vec();
    预期.sort_unstable();
    for (路径, 条件) in [
        ("/commands", "enablement"),
        ("/menus/commandPalette", "when"),
    ] {
        let 列表 = 记录列表(贡献, 路径)?;
        let mut 标识 = Vec::new();
        for 条目 in 列表 {
            let 名称 = 条目["command"]
                .as_str()
                .ok_or_else(|| 工具错误::新(format!("清单命令标识必须是字符串：{路径}")))?;
            要求(
                条目[条件] == json!("isLinux"),
                &format!("{名称} 的 {条件} 必须为 isLinux"),
            )?;
            标识.push(名称);
        }
        标识.sort_unstable();
        要求(标识 == 预期, &format!("{路径} 必须恰好声明五个已实现命令"))?;
    }
    要求(
        清单.pointer("/capabilities/virtualWorkspaces") == Some(&json!(false)),
        "不支持虚拟工作区",
    )?;
    要求(
        清单.pointer("/capabilities/untrustedWorkspaces/supported") == Some(&json!(false)),
        "不支持受限模式",
    )?;
    要求(
        清单["extensionKind"] == json!(["ui"]),
        "扩展只能运行在本机 UI 侧",
    )
}

/// 从指定项目读取实际清单，使完整检查覆盖未提交的清单修改。
pub fn 验证设置(根目录: &Path) -> 结果<()> {
    let 清单: Value = serde_json::from_slice(&std::fs::read(根目录.join("package.json"))?)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    校验清单(&清单)?;
    println!("清单：默认设置、命令与能力声明验证通过");
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 当前清单() -> Value {
        serde_json::from_str(include_str!("../../package.json")).unwrap()
    }

    #[test]
    fn 当前清单通过且记录顺序不影响校验() {
        let mut 清单 = 当前清单();
        校验清单(&清单).unwrap();
        for 路径 in ["/contributes/commands", "/contributes/menus/commandPalette"] {
            清单
                .pointer_mut(路径)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .reverse();
        }
        校验清单(&清单).unwrap();
    }

    #[test]
    fn 错误默认设置命令与能力逐项拒绝() {
        for (路径, 值) in [
            ("/contributes/configurationDefaults", Value::Null),
            ("/contributes/themes", Value::Null),
            ("/contributes/productIconThemes", json!([])),
            (
                "/contributes/configurationDefaults/workbench.preferredLightColorTheme",
                json!("未注册"),
            ),
            (
                "/contributes/configurationDefaults/workbench.preferredDarkColorTheme",
                Value::Null,
            ),
            (
                "/contributes/configurationDefaults/workbench.preferredHighContrastLightColorTheme",
                json!(1),
            ),
            (
                "/contributes/configurationDefaults/workbench.preferredHighContrastColorTheme",
                json!("未注册"),
            ),
            (
                "/contributes/configurationDefaults/window.menuBarVisibility",
                json!("hidden"),
            ),
            (
                "/contributes/configurationDefaults/editor.fontFamily",
                json!("monospace"),
            ),
            (
                "/contributes/configuration/properties/adwcode.界面字体/default",
                Value::Null,
            ),
            ("/contributes/commands", Value::Null),
            (
                "/contributes/commands/0/command",
                json!("adwcode.应用推荐设置"),
            ),
            ("/contributes/commands/0/enablement", json!("true")),
            ("/contributes/menus/commandPalette/0/when", Value::Null),
            ("/contributes/menus/commandPalette/0/command", Value::Null),
            ("/capabilities/virtualWorkspaces", json!(true)),
            ("/capabilities/untrustedWorkspaces/supported", json!(true)),
            ("/extensionKind", json!(["workspace"])),
        ] {
            let mut 清单 = 当前清单();
            *清单.pointer_mut(路径).unwrap() = 值;
            assert!(校验清单(&清单).is_err(), "错误值未拒绝：{路径}");
        }
        let mut 清单 = 当前清单();
        清单["contributes"]["configurationDefaults"]["workbench.iconTheme"] = Value::Null;
        assert!(校验清单(&清单).is_err());
        for 路径 in ["/contributes/commands", "/contributes/menus/commandPalette"] {
            let mut 清单 = 当前清单();
            let 列表 = 清单.pointer_mut(路径).unwrap().as_array_mut().unwrap();
            列表.push(列表[0].clone());
            assert!(校验清单(&清单).is_err());
        }
    }

    #[test]
    fn 验证读取磁盘实际内容() {
        let 根 = tempfile::tempdir().unwrap();
        assert!(验证设置(根.path()).is_err());
        std::fs::write(根.path().join("package.json"), 当前清单().to_string()).unwrap();
        验证设置(根.path()).unwrap();
        std::fs::write(根.path().join("package.json"), "{").unwrap();
        assert!(验证设置(根.path()).is_err());
        std::fs::write(根.path().join("package.json"), "{}").unwrap();
        assert!(验证设置(根.path()).is_err());
    }
}
