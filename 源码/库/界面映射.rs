// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 工作台颜色有序规则和受限表达式解释器；规则在编译时嵌入，不执行外部代码。

use crate::有序映射::有序映射;
use crate::语法映射::{加载样式方案, 样式信息};
use crate::调色板::{叠加颜色, 合成透明颜色, 色阶, 解析颜色, 调色板对象};
use crate::错误::{工具错误, 结果};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

const 规则文本: &str = include_str!("界面映射/规则.json");
const 许可: &str = "AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later";
static 已解析规则: OnceLock<std::result::Result<Vec<规则>, String>> = OnceLock::new();

#[derive(Clone, Copy)]
enum 条件 {
    深色,
    高对比度,
    浅色标签,
    彩色状态栏,
}
impl 条件 {
    fn 解析(文本: &str) -> 结果<Self> {
        match 文本 {
            "深色" => Ok(Self::深色),
            "高对比度" => Ok(Self::高对比度),
            "浅色标签" => Ok(Self::浅色标签),
            "彩色状态栏" => Ok(Self::彩色状态栏),
            _ => Err(工具错误::新(format!("未知界面条件：{文本}"))),
        }
    }
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

enum 表达式 {
    空,
    常量(String),
    角色(String),
    透明(Box<Self>, f64),
    叠加(Box<Self>, f64, String),
    色调(String, usize, usize),
    样式(String, bool, Box<Self>),
    条件(条件, Box<Self>, Box<Self>),
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
impl 表达式 {
    fn 可为空(&self) -> bool {
        match self {
            Self::空 => true,
            Self::条件(_, 是, 否) => 是.可为空() || 否.可为空(),
            _ => false,
        }
    }
    fn 必需(&self, 上下文: &上下文<'_>) -> 结果<String> {
        self.求值(上下文)?
            .ok_or_else(|| 工具错误::新("颜色表达式不得为空"))
    }
    fn 求值(&self, 上下文: &上下文<'_>) -> 结果<Option<String>> {
        let 调色板 = 上下文.调色板;
        Ok(Some(match self {
            Self::空 => return Ok(None),
            Self::常量(值) => 值.clone(),
            Self::角色(名称) => 角色值(调色板, 名称)?,
            Self::透明(颜色, 比例) => 合成透明颜色(&颜色.必需(上下文)?, *比例)?,
            Self::叠加(颜色, 比例, 表面) => {
                let 前景 = 合成透明颜色(&颜色.必需(上下文)?, *比例)?;
                叠加颜色(&前景, &角色值(调色板, 表面)?)?
            }
            Self::色调(名称, 深, 浅) => {
                let 级 = if 调色板.mode == "dark" { *深 } else { *浅 };
                色阶(名称)
                    .and_then(|色阶| 色阶.get(级 - 1).copied())
                    .ok_or_else(|| 工具错误::新("未知色阶或级数"))?
                    .to_string()
            }
            Self::样式(名称, 前景, 回退) => {
                let 回退 = 回退.必需(上下文)?;
                上下文
                    .样式
                    .get(名称)
                    .and_then(|样式| {
                        if *前景 {
                            样式.foreground.clone()
                        } else {
                            样式.background.clone()
                        }
                    })
                    .filter(|值| !值.is_empty())
                    .unwrap_or(回退)
            }
            Self::条件(条件, 是, 否) => {
                return if 条件.满足(上下文) {
                    是.求值(上下文)
                } else {
                    否.求值(上下文)
                };
            }
        }))
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
        值: 表达式,
        条件: Option<条件>,
    },
}

fn 对象<'a>(值: &'a Value, 必需: &[&str], 可选: &[&str]) -> 结果<&'a Map<String, Value>> {
    let 表 = 值
        .as_object()
        .ok_or_else(|| 工具错误::新("界面规则字段必须是对象"))?;
    if 必需.iter().any(|键| !表.contains_key(*键))
        || 表
            .keys()
            .any(|键| !必需.contains(&键.as_str()) && !可选.contains(&键.as_str()))
    {
        return Err(工具错误::新("界面规则含未知字段或缺少必需字段"));
    }
    Ok(表)
}
fn 文本<'a>(表: &'a Map<String, Value>, 键: &str) -> 结果<&'a str> {
    表.get(键)
        .and_then(Value::as_str)
        .ok_or_else(|| 工具错误::新(format!("{键} 必须是字符串")))
}
fn 名称(表: &Map<String, Value>, 键: &str) -> 结果<String> {
    let 值 = 文本(表, 键)?;
    if 值.is_empty() {
        return Err(工具错误::新(format!("{键} 不得为空")));
    }
    Ok(值.to_owned())
}
fn 子表达式(值: &Value, 深度: usize) -> 结果<表达式> {
    let 值 = 解析表达式(值, 深度)?;
    if 值.可为空() {
        return Err(工具错误::新("此处颜色表达式不得为空"));
    }
    Ok(值)
}
fn 解析表达式(值: &Value, 深度: usize) -> 结果<表达式> {
    if 深度 > 32 {
        return Err(工具错误::新("界面颜色表达式嵌套过深"));
    }
    let 类型 = 值
        .get("类型")
        .and_then(Value::as_str)
        .ok_or_else(|| 工具错误::新("颜色表达式缺少类型"))?;
    let 字段: &[&str] = match 类型 {
        "空" => &[],
        "常量" => &["值"],
        "角色" => &["名称"],
        "透明" => &["颜色", "比例"],
        "叠加" => &["颜色", "比例", "表面"],
        "色调" => &["名称", "深", "浅"],
        "样式" => &["名称", "前景", "回退"],
        "条件" => &["条件", "是", "否"],
        _ => return Err(工具错误::新(format!("未知颜色运算：{类型}"))),
    };
    let mut 必需 = vec!["类型"];
    必需.extend_from_slice(字段);
    let 表 = 对象(值, &必需, &[])?;
    let 比例 = || -> 结果<f64> {
        表["比例"]
            .as_f64()
            .filter(|值| 值.is_finite() && (0.0..=1.0).contains(值))
            .ok_or_else(|| 工具错误::新("颜色比例必须是 0..1 的有限数"))
    };
    let 级数 = |键: &str| -> 结果<usize> {
        表[键]
            .as_u64()
            .filter(|级| (1..=5).contains(级))
            .map(|级| 级 as usize)
            .ok_or_else(|| 工具错误::新("色阶级数必须为 1..5 整数"))
    };
    Ok(match 类型 {
        "空" => 表达式::空,
        "常量" => {
            let 值 = 名称(表, "值")?;
            解析颜色(&值)?;
            表达式::常量(值)
        }
        "角色" => 表达式::角色(名称(表, "名称")?),
        "透明" => 表达式::透明(Box::new(子表达式(&表["颜色"], 深度 + 1)?), 比例()?),
        "叠加" => 表达式::叠加(
            Box::new(子表达式(&表["颜色"], 深度 + 1)?),
            比例()?,
            名称(表, "表面")?,
        ),
        "色调" => {
            let 名 = 名称(表, "名称")?;
            if 色阶(&名).is_none() {
                return Err(工具错误::新("未知色阶"));
            }
            表达式::色调(名, 级数("深")?, 级数("浅")?)
        }
        "样式" => 表达式::样式(
            名称(表, "名称")?,
            表["前景"]
                .as_bool()
                .ok_or_else(|| 工具错误::新("前景 必须是布尔值"))?,
            Box::new(子表达式(&表["回退"], 深度 + 1)?),
        ),
        "条件" => 表达式::条件(
            条件::解析(文本(表, "条件")?)?,
            Box::new(解析表达式(&表["是"], 深度 + 1)?),
            Box::new(解析表达式(&表["否"], 深度 + 1)?),
        ),
        _ => unreachable!("已校验运算"),
    })
}

fn 解析规则(文本值: &str) -> 结果<Vec<规则>> {
    let 值: Value = serde_json::from_str(文本值)
        .map_err(|错误| 工具错误::新(format!("界面规则 JSON 解析失败：{错误}")))?;
    let 根 = 对象(&值, &["格式", "许可", "版权", "说明", "规则"], &[])?;
    名称(根, "版权")?;
    if 根["格式"].as_u64() != Some(1) || 文本(根, "许可")? != 许可 {
        return Err(工具错误::新("不支持的界面规则格式或许可"));
    }
    文本(根, "说明")?;
    let 行 = 根["规则"]
        .as_array()
        .filter(|行| !行.is_empty() && 行.len() <= 4096)
        .ok_or_else(|| 工具错误::新("界面规则必须是非空数组且不超过 4096 项"))?;
    行.iter()
        .map(|值| {
            let 操作 = 值
                .get("操作")
                .and_then(Value::as_str)
                .ok_or_else(|| 工具错误::新("规则缺少操作"))?;
            match 操作 {
                "注释" => {
                    let 表 = 对象(值, &["操作", "说明"], &[])?;
                    文本(表, "说明")?;
                    Ok(规则::注释)
                }
                "终端" => {
                    对象(值, &["操作"], &[])?;
                    Ok(规则::终端)
                }
                "放" | "放可选" | "覆盖" => {
                    let 表 = 对象(值, &["操作", "键", "值"], &["条件"])?;
                    let 条件 = 表
                        .get("条件")
                        .map(|值| {
                            条件::解析(
                                值.as_str()
                                    .ok_or_else(|| 工具错误::新("条件必须是字符串"))?,
                            )
                        })
                        .transpose()?;
                    let 操作 = match 操作 {
                        "放" => 操作::放,
                        "放可选" => 操作::放可选,
                        _ => 操作::覆盖,
                    };
                    let 表达式 = if matches!(操作, 操作::放可选) {
                        解析表达式(&表["值"], 0)?
                    } else {
                        子表达式(&表["值"], 0)?
                    };
                    Ok(规则::颜色 {
                        操作,
                        键: 名称(表, "键")?,
                        值: 表达式,
                        条件,
                    })
                }
                _ => Err(工具错误::新(format!("未知界面规则操作：{操作}"))),
            }
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
            &serde_json::json!({"格式":1,"许可":许可,"版权":"测试贡献者","说明":"测试","规则":行})
                .to_string(),
        )
    }

    #[test]
    fn 非法规则与表达式严格拒绝() {
        let 行 = serde_json::json!({"操作":"放","键":"测试","值":{"类型":"常量","值":"#123456"}});
        assert!(解析行(serde_json::json!([行])).is_ok());
        for 值 in [
            serde_json::json!({"类型":"执行","值":"任意代码"}),
            serde_json::json!({"类型":"常量","值":"错误颜色"}),
            serde_json::json!({"类型":"常量","值":"#123456","多余":true}),
            serde_json::json!({"类型":"角色","名称":""}),
            serde_json::json!({"类型":"透明","颜色":{"类型":"常量","值":"#123456"},"比例":-0.1}),
            serde_json::json!({"类型":"透明","颜色":{"类型":"空"},"比例":0.5}),
            serde_json::json!({"类型":"叠加","颜色":{"类型":"角色","名称":"fg"},"比例":1.1,"表面":"bg_view"}),
            serde_json::json!({"类型":"色调","名称":"blue","深":0,"浅":5}),
            serde_json::json!({"类型":"色调","名称":"blue","深":5,"浅":6}),
            serde_json::json!({"类型":"色调","名称":"无此色阶","深":1,"浅":5}),
            serde_json::json!({"类型":"样式","名称":"def:type","前景":"true","回退":{"类型":"常量","值":"#123456"}}),
            serde_json::json!({"类型":"条件","条件":"陌生条件","是":{"类型":"空"},"否":{"类型":"空"}}),
        ] {
            let mut 坏 = 行.clone();
            坏["值"] = 值;
            assert!(解析行(serde_json::json!([坏])).is_err(), "{坏}");
        }
        for 坏 in [
            serde_json::json!([]),
            serde_json::json!([{}]),
            serde_json::json!([{"操作":"终端","值":null}]),
            serde_json::json!([{"操作":"放","键":"","值":{"类型":"空"}}]),
        ] {
            assert!(解析行(坏).is_err());
        }
        let mut 深 = serde_json::json!({"类型":"常量","值":"#123456"});
        for _ in 0..34 {
            深 = serde_json::json!({"类型":"条件","条件":"深色","是":深,"否":{"类型":"常量","值":"#123456"}});
        }
        assert!(解析行(serde_json::json!([{"操作":"放","键":"测试","值":深}])).is_err());
        let mut 根: Value = serde_json::from_str(规则文本).unwrap();
        根["格式"] = serde_json::json!(2);
        assert!(解析规则(&根.to_string()).is_err());
        根["格式"] = serde_json::json!(1);
        根["新增"] = serde_json::json!(true);
        assert!(解析规则(&根.to_string()).is_err());
        assert!(解析规则("{").is_err());
    }

    #[test]
    fn 规则覆盖省略终端与样式回退保持顺序() {
        let 根目录 = Path::new(env!("CARGO_MANIFEST_DIR"));
        let 调色板 =
            调色板对象::新("dark", "blue", false, 编辑器颜色(根目录, "dark").unwrap()).unwrap();
        let 样式 = BTreeMap::new();
        let 上下文 = 上下文 {
            调色板: &调色板,
            样式: &样式,
            彩色状态栏: false,
        };
        let 规则 = 解析行(serde_json::json!([
            {"操作":"放可选","键":"首项","值":{"类型":"空"}},
            {"操作":"放","键":"回退","值":{"类型":"样式","名称":"缺失样式","前景":true,"回退":{"类型":"常量","值":"#112233"}}},
            {"操作":"终端"},
            {"操作":"覆盖","键":"首项","值":{"类型":"常量","值":"#445566"}},
            {"操作":"覆盖","键":"末项","值":{"类型":"常量","值":"#778899"}},
            {"操作":"放","键":"不应出现","条件":"彩色状态栏","值":{"类型":"常量","值":"#000000"}}
        ])).unwrap();
        let 结果 = 应用规则(&规则, &上下文).unwrap();
        assert_eq!(结果.条目().len(), 19);
        assert_eq!(结果.条目()[0], ("首项".to_owned(), "#445566".to_owned()));
        assert_eq!(结果.条目()[1], ("回退".to_owned(), "#112233".to_owned()));
        assert_eq!(结果.条目().last().unwrap().0, "末项");
        for (键, 值) in crate::调色板::终端颜色() {
            assert_eq!(结果.取(键), Some(值));
        }
        for 值 in [
            serde_json::json!({"类型":"角色","名称":"无此角色"}),
            serde_json::json!({"类型":"叠加","颜色":{"类型":"常量","值":"#123456"},"比例":0.5,"表面":"无此表面"}),
        ] {
            let 规则 = 解析行(serde_json::json!([{"操作":"放","键":"错误","值":值}])).unwrap();
            assert!(应用规则(&规则, &上下文).is_err());
        }
    }
}
