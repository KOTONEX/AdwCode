// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 工作台颜色有序映射表；复杂计算使用固定的 Rust 函数。

#[path = "界面映射/计算.rs"]
mod 计算;

use crate::有序映射::有序映射;
use crate::语法映射::{加载样式方案, 样式信息};
use crate::调色板::{叠加颜色, 合成透明颜色, 色阶, 解析颜色, 调色板对象};
use crate::错误::{工具错误, 结果};
#[cfg(test)]
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

const 规则文本: &str = include_str!("界面映射/规则.json");
const 许可: &str = "AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later";
static 已解析规则: OnceLock<std::result::Result<Vec<规则>, String>> = OnceLock::new();

#[derive(Clone, Copy, serde::Deserialize)]
enum 条件 {
    深色,
    高对比度,
    浅色标签,
    彩色状态栏,
}
impl 条件 {
    fn 满足(self, 上下文: &上下文<'_>) -> bool {
        let 深色 = 上下文.调色板.mode == "dark";
        match self {
            Self::深色 => 深色,
            Self::高对比度 => 上下文.调色板.high_contrast,
            Self::浅色标签 => !深色 && !上下文.调色板.high_contrast,
            Self::彩色状态栏 => 上下文.彩色状态栏,
        }
    }
}

enum 颜色值 {
    常量(String),
    角色(String),
    计算(fn(&上下文<'_>) -> 结果<Option<String>>),
}
struct 上下文<'a> {
    调色板: &'a 调色板对象,
    样式: &'a BTreeMap<String, 样式信息>,
    彩色状态栏: bool,
}

fn 角色值(调色板: &调色板对象, 名称: &str) -> 结果<String> {
    调色板
        .取可选(名称)
        .map(str::to_owned)
        .ok_or_else(|| 工具错误::新(format!("未知调色板角色：{名称}")))
}
impl 颜色值 {
    fn 求值(&self, 上下文: &上下文<'_>) -> 结果<Option<String>> {
        match self {
            Self::常量(值) => Ok(Some(值.clone())),
            Self::角色(名称) => Ok(Some(角色值(上下文.调色板, 名称)?)),
            Self::计算(函数) => 函数(上下文),
        }
    }
    fn 必需(&self, 上下文: &上下文<'_>) -> 结果<String> {
        self.求值(上下文)?
            .ok_or_else(|| 工具错误::新("此颜色条目不得为空"))
    }
}

enum 操作 {
    放,
    放可选,
    覆盖,
}
enum 规则 {
    注释,
    终端,
    颜色 {
        操作: 操作,
        键: String,
        值: 颜色值,
        条件: Option<条件>,
    },
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct 映射清单 {
    格式: u64,
    许可: String,
    版权: String,
    #[serde(rename = "说明")]
    _说明: String,
    规则: Vec<映射行>,
}

// 缺少字段与显式 null 不混淆；出现的可选字段仍必须通过类型校验。
fn 读取可选字段<'de, D, T>(读取器: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(读取器).map(Some)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct 映射行 {
    操作: String,
    #[serde(default, deserialize_with = "读取可选字段")]
    键: Option<String>,
    #[serde(default, deserialize_with = "读取可选字段")]
    角色: Option<String>,
    #[serde(default, deserialize_with = "读取可选字段")]
    常量: Option<String>,
    #[serde(default, deserialize_with = "读取可选字段")]
    计算: Option<String>,
    #[serde(default, deserialize_with = "读取可选字段")]
    条件: Option<条件>,
    #[serde(default, deserialize_with = "读取可选字段")]
    说明: Option<String>,
}
fn 读取颜色(行: &映射行) -> 结果<颜色值> {
    match (&行.角色, &行.常量, &行.计算) {
        (Some(名), None, None) if !名.is_empty() => Ok(颜色值::角色(名.clone())),
        (None, Some(值), None) => {
            解析颜色(值)?;
            Ok(颜色值::常量(值.clone()))
        }
        (None, None, Some(名)) => Ok(颜色值::计算(计算::查找计算(名)?)),
        _ => Err(工具错误::新(
            "颜色条目必须且只能指定非空角色、常量或计算之一",
        )),
    }
}
fn 解析规则(文本值: &str) -> 结果<Vec<规则>> {
    let 根: 映射清单 = serde_json::from_str(文本值)
        .map_err(|错误| 工具错误::新(format!("界面映射 JSON 解析失败：{错误}")))?;
    if 根.格式 != 2 || 根.许可 != 许可 || 根.版权.is_empty() {
        return Err(工具错误::新("不支持的界面映射格式、许可或版权"));
    }
    if 根.规则.is_empty() || 根.规则.len() > 4096 {
        return Err(工具错误::新("界面映射必须是非空数组且不超过 4096 项"));
    }
    根.规则
        .into_iter()
        .map(|行| match 行.操作.as_str() {
            "注释" | "终端" => {
                if 行.键.is_some()
                    || 行.角色.is_some()
                    || 行.常量.is_some()
                    || 行.计算.is_some()
                    || 行.条件.is_some()
                {
                    return Err(工具错误::新("注释或终端条目含颜色字段"));
                }
                match 行.操作.as_str() {
                    "注释" if 行.说明.is_some() => Ok(规则::注释),
                    "终端" if 行.说明.is_none() => Ok(规则::终端),
                    _ => Err(工具错误::新("注释或终端条目字段错误")),
                }
            }
            "放" | "放可选" | "覆盖" => {
                if 行.说明.is_some() {
                    return Err(工具错误::新("颜色条目含注释字段"));
                }
                let 值 = 读取颜色(&行)?;
                let 键 = 行
                    .键
                    .filter(|键| !键.is_empty())
                    .ok_or_else(|| 工具错误::新("颜色键不得为空"))?;
                let 操作 = match 行.操作.as_str() {
                    "放" => 操作::放,
                    "放可选" => 操作::放可选,
                    _ => 操作::覆盖,
                };
                Ok(规则::颜色 {
                    操作,
                    键,
                    值,
                    条件: 行.条件,
                })
            }
            _ => Err(工具错误::新(format!("未知界面映射操作：{}", 行.操作))),
        })
        .collect()
}

fn 应用规则(规则: &[规则], 上下文: &上下文<'_>) -> 结果<有序映射> {
    let mut 颜色 = 颜色表::新();
    for 规则 in 规则 {
        match 规则 {
            规则::注释 => {}
            规则::终端 => {
                for (键, 值) in crate::调色板::终端颜色() {
                    颜色.放(键, 值);
                }
            }
            规则::颜色 {
                操作, 键, 值, 条件
            } => {
                if 条件.is_some_and(|条件| !条件.满足(上下文)) {
                    continue;
                }
                match 操作 {
                    操作::放 => 颜色.放(键, 值.必需(上下文)?),
                    操作::放可选 => 颜色.放可选(键, 值.求值(上下文)?),
                    操作::覆盖 => 颜色.覆盖(键, 值.必需(上下文)?),
                }
            }
        }
    }
    Ok(颜色.完成())
}

/// 生成有序工作台颜色；省略空值，保持全部登记及原位覆盖语义。
///
/// # Errors
/// 规则无效、角色缺失或样式方案不可读取时返回错误。
pub fn 生成界面颜色(
    调色板: &调色板对象,
    彩色状态栏: bool,
    根目录: &Path,
) -> 结果<有序映射> {
    let 规则 = 已解析规则
        .get_or_init(|| 解析规则(规则文本).map_err(|错误| 错误.to_string()))
        .as_ref()
        .map_err(工具错误::新)?;
    let 样式 = 加载样式方案(根目录, &调色板.mode)?.styles;
    应用规则(
        规则,
        &上下文 {
            调色板,
            样式: &样式,
            彩色状态栏,
        },
    )
}

/// 收集颜色条目；None 表示普通主题中不定义的键，最后统一过滤。
struct 颜色表 {
    条目: Vec<(String, Option<String>)>,
}

impl 颜色表 {
    fn 新() -> Self {
        Self { 条目: Vec::new() }
    }

    fn 放(&mut self, 键: &str, 值: impl Into<String>) {
        self.条目.push((键.to_string(), Some(值.into())));
    }

    fn 放可选(&mut self, 键: &str, 值: Option<String>) {
        self.条目.push((键.to_string(), 值));
    }

    /// 对应 Python 的 `dict.update`：已有键原位替换，缺失键追加到末尾。
    fn 覆盖(&mut self, 键: &str, 值: impl Into<String>) {
        let 值 = 值.into();
        if let Some(项) = self.条目.iter_mut().find(|(已有, _)| *已有 == 键) {
            项.1 = Some(值);
        } else {
            self.条目.push((键.to_string(), Some(值)));
        }
    }

    fn 完成(self) -> 有序映射 {
        let mut 输出 = 有序映射::新();
        for (键, 值) in self.条目 {
            if let Some(值) = 值 {
                输出.放(键, 值);
            }
        }
        输出
    }
}

#[cfg(test)]
mod 测试 {
    use std::path::Path;

    use super::*;
    use crate::语法映射::编辑器颜色;

    #[test]
    fn 与金样例一致() {
        let 路径 = concat!(env!("CARGO_MANIFEST_DIR"), "/测试/Rust/界面样例.json");
        let 文本 = std::fs::read_to_string(路径).expect("读取界面金样例");
        let 样例: serde_json::Value = serde_json::from_str(&文本).expect("解析界面金样例");
        let 根目录 = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (场景, 期望) in 样例.as_object().expect("样例为对象") {
            let mode = if 场景.starts_with("dark") {
                "dark"
            } else {
                "light"
            };
            let high_contrast = 场景.contains("高对比度");
            let 彩色状态栏 = 场景.contains("彩色状态栏");
            let 方案 = 编辑器颜色(根目录, mode).expect("构建编辑器方案");
            let 调色板 = 调色板对象::新(mode, "blue", high_contrast, 方案).expect("构建调色板");
            let 颜色 = 生成界面颜色(&调色板, 彩色状态栏, 根目录).expect("生成界面颜色");
            let 期望表 = 期望.as_object().expect("场景为对象");
            assert_eq!(颜色.条目().len(), 期望表.len(), "场景 {场景} 的键数量");
            for ((键, 值), (期望键, 期望值)) in 颜色.条目().iter().zip(期望表) {
                assert_eq!(键, 期望键, "场景 {场景} 的键顺序");
                assert_eq!(
                    值,
                    期望值.as_str().expect("颜色为字符串"),
                    "场景 {场景} 的 {键}"
                );
            }
        }
    }
    fn 解析行(行: Value) -> 结果<Vec<规则>> {
        解析规则(
            &serde_json::json!({"格式":2,"许可":许可,"版权":"测试贡献者","说明":"测试","规则":行})
                .to_string(),
        )
    }
    #[test]
    fn 平坦映射拒绝无效字段与计算() {
        for 行 in [
            serde_json::json!({"操作":"放","键":"测试"}),
            serde_json::json!({"操作":"放","键":"测试","角色":null}),
            serde_json::json!({"操作":"放","键":"测试","角色":"fg","条件":null}),
            serde_json::json!({"操作":"放","键":"测试","角色":"fg","常量":"#123456"}),
            serde_json::json!({"操作":"放","键":"测试","常量":"错误颜色"}),
            serde_json::json!({"操作":"放","键":"测试","角色":""}),
            serde_json::json!({"操作":"放","键":"测试","计算":"任意代码"}),
            serde_json::json!({"操作":"放","键":"测试","值":{"类型":"角色","名称":"fg"}}),
            serde_json::json!({"操作":"终端","常量":"#123456"}),
            serde_json::json!({"操作":"放","键":"测试","角色":"fg","条件":"未知"}),
        ] {
            assert!(解析行(serde_json::json!([行])).is_err(), "{行}");
        }
        assert!(解析行(serde_json::json!([])).is_err());
        assert!(解析规则("{").is_err());
        let mut 根: Value = serde_json::from_str(规则文本).unwrap();
        根["格式"] = serde_json::json!(1);
        assert!(解析规则(&根.to_string()).is_err());
        根["格式"] = serde_json::json!(2);
        根["新增"] = serde_json::json!(true);
        assert!(解析规则(&根.to_string()).is_err());
    }
    #[test]
    fn 覆盖省略终端与样式回退保持顺序() {
        let 根 = Path::new(env!("CARGO_MANIFEST_DIR"));
        let 调色板 =
            调色板对象::新("dark", "blue", false, 编辑器颜色(根, "dark").unwrap()).unwrap();
        let 样式 = BTreeMap::new();
        let 上下文 = 上下文 {
            调色板: &调色板,
            样式: &样式,
            彩色状态栏: false,
        };
        let 规则 = 解析行(serde_json::json!([
            {"操作":"放可选","键":"首项","计算":"高对比度_contrast_border_否则_省略"},
            {"操作":"放","键":"回退","常量":"#112233"},
            {"操作":"终端"},
            {"操作":"覆盖","键":"首项","常量":"#445566"},
            {"操作":"覆盖","键":"末项","常量":"#778899"},
            {"操作":"放","键":"不应出现","条件":"彩色状态栏","常量":"#000000"}
        ]))
        .unwrap();
        let 结果 = 应用规则(&规则, &上下文).unwrap();
        assert_eq!(结果.条目().len(), 19);
        assert_eq!(结果.条目()[0], ("首项".to_owned(), "#445566".to_owned()));
        assert_eq!(结果.条目()[1], ("回退".to_owned(), "#112233".to_owned()));
        assert_eq!(结果.条目().last().unwrap().0, "末项");
        for (键, 值) in crate::调色板::终端颜色() {
            assert_eq!(结果.取(键), Some(值));
        }
        let 回退 = 计算::查找计算("样式_def:type_回退_fg").unwrap()(&上下文).unwrap();
        assert_eq!(回退, Some(角色值(&调色板, "fg").unwrap()));
        let 坏 = 解析行(serde_json::json!([{"操作":"放","键":"错误","角色":"无此角色"}])).unwrap();
        assert!(应用规则(&坏, &上下文).is_err());
    }
}
