// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 产品图标字体生成：自有 SVG 字形与固定版本的导入字形。
//!
//! 由 `产品图标/生成自有字形.py` 与 `产品图标/生成导入字形.py` 迁移而来：
//! 用 kurbo 解析几何、flo_curves 处理布尔轮廓、write-fonts 写出 TTF，
//! 运行期不再依赖 fontTools 与 skia-pathops。生成结果确定：同一份输入
//! 重复运行得到字节一致的字体（时间戳固定，表格顺序稳定）。
//!
//! 与 Python 版一致的语义：16 × 16 画布、64 单位/像素、基线 896、
//! 字形名 `uni<码点>`、按许可证分组生成字体、来源哈希校验、
//! 字重内缩（外轮廓收缩、内孔扩张）、`渲染图标/` 预览导出。

#[path = "几何.rs"]
mod 几何;
#[path = "字体封装.rs"]
mod 字体封装;
#[cfg(test)]
#[path = "测试.rs"]
mod 测试;
#[path = "矢量解析.rs"]
mod 矢量解析;
#[path = "预览.rs"]
mod 预览;

use self::几何::{子路径, 应用字重内缩};
use self::字体封装::{字体描述, 生成字体字节};
use self::矢量解析::{收集path元素, 绘制svg策略, 解析xml};
use self::预览::导出预览;
use crate::摘要::sha256十六进制;
use crate::错误::{工具错误, 结果};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// SVG 命名空间；非该命名空间的元素不参与绘制。
const SVG命名空间: &str = "http://www.w3.org/2000/svg";
/// 字体设计单位：每个 SVG 像素 64 单位。
const 每像素单位: f64 = 64.0;
/// 基线与画布下沿：y 字体 = 896 - 64 × y 画布。
const 基线: f64 = 896.0;
/// 固定时间戳（1904 纪元秒），使生成结果可复现。
const 固定时间戳: i64 = 3_863_548_800;
/// 字形前进宽度。
const 前进宽度: u16 = 1024;
/// 路径与几何属性的坐标幅度上限。
const 坐标上限: f64 = 10_000.0;
/// 单条曲线展平的最大递归深度。
const 展平最大深度: u32 = 12;
/// XML 允许的最大元素嵌套深度。
const 最大嵌套深度: usize = 256;

/// `adwcode 生成自有字形` 的入口。
pub fn 自有入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    if let Some(其他) = 参数.first() {
        return Err(工具错误::新(format!("未知参数：{其他}")));
    }
    let 目录 = 根目录.join("产品图标").join("符号图标");
    let mut 源文件: Vec<PathBuf> = std::fs::read_dir(&目录)
        .map_err(|错误| 工具错误::带来源(format!("无法读取 {}", 目录.display()), 错误))?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|项| 项.path())
        .filter(|路径| 路径.extension().is_some_and(|扩展| 扩展 == "svg"))
        .collect();
    源文件.sort_by_key(|路径| 路径.file_name().map(|名| 名.to_os_string()));
    if 源文件.is_empty() {
        return Err(工具错误::新("自有字形来源不能为空"));
    }
    let mut 名称表 = vec![".notdef".to_string()];
    let mut 轮廓表: Vec<Vec<子路径>> = vec![Vec::new()];
    let mut 码点表: Vec<(u16, usize)> = Vec::new();
    for 文件 in &源文件 {
        let 文件名 = 文件
            .file_name()
            .map_or_else(String::new, |名| 名.to_string_lossy().to_string());
        crate::文件事务::校验普通路径(文件)?;
        let 文本 = std::fs::read_to_string(文件).map_err(|错误| {
            工具错误::带来源(format!("无法读取 {}", 文件.display()), 错误)
        })?;
        let 根 = 解析xml(&文本, &文件名)?;
        let mut 轮廓 = Vec::new();
        收集path元素(&根, &文件名, &mut 轮廓)?;
        if 轮廓.iter().all(|项| 项.段.is_empty()) {
            return Err(工具错误::新(format!("字形为空：{文件名}")));
        }
        let 词干 = 文件
            .file_stem()
            .map_or_else(String::new, |名| 名.to_string_lossy().to_string());
        let 码点 = u16::from_str_radix(&词干, 16)
            .map_err(|_| 工具错误::新(format!("{文件名} 的文件名不是十六进制码点")))?;
        名称表.push(format!("uni{}", 词干.to_uppercase()));
        码点表.push((码点, 轮廓表.len()));
        轮廓表.push(轮廓);
    }
    let 描述 = 字体描述 {
        族名: "AdwCode Symbols",
        样式名: "Regular",
        唯一标识: "AdwCodeSymbols-Regular",
        全名: "AdwCode Symbols",
        ps名: "AdwCodeSymbols-Regular",
        版本: "Version 1.0",
        版权: "2026 AdwCode 贡献者",
        许可: "AGPL-3.0-or-later OR CC-BY-SA-4.0+；许可声明见 产品图标/LICENSE",
        许可地址: "https://github.com/KOTONEX/AdwCode/blob/%E4%B8%BB%E7%BA%BF/产品图标/LICENSE",
    };
    let 字节 = 生成字体字节(&名称表, &轮廓表, &码点表, &描述)?;
    crate::文件事务::写入批次(&[(根目录.join("产品图标/adwcode-符号.ttf"), Some(字节))])?;
    println!("已生成 {} 个单色字形", 源文件.len());
    Ok(())
}

/// `adwcode 生成导入字形` 的入口。
pub fn 导入入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    if let Some(其他) = 参数.first() {
        return Err(工具错误::新(format!("未知参数：{其他}")));
    }
    let 图标目录 = 根目录.join("产品图标");
    let 来源文本 = std::fs::read_to_string(图标目录.join("来源.json"))
        .map_err(|错误| 工具错误::带来源("无法读取 产品图标/来源.json".to_string(), 错误))?;
    let 来源: Value = serde_json::from_str(&来源文本)
        .map_err(|错误| 工具错误::新(format!("产品图标/来源.json 解析失败：{错误}")))?;
    let 字体列表 = 来源["fonts"]
        .as_array()
        .ok_or_else(|| 工具错误::新("产品图标/来源.json 缺少 fonts"))?;
    if 字体列表.is_empty() {
        return Err(工具错误::新("来源.json 的 fonts 不能为空"));
    }
    let mut 字体标识 = std::collections::HashSet::new();
    let mut 操作 = Vec::new();
    let mut 消息 = Vec::new();
    for 字体 in 字体列表 {
        let id = 字符串字段(字体, "id")?;
        if !字体标识.insert(id.clone()) {
            return Err(工具错误::新("来源.json 字体标识重复"));
        }
        let id路径 = 校验相对路径(&id, "fonts[].id")?;
        let 族名 = 字符串字段(字体, "family")?;
        let 输出 = 字符串字段(字体, "output")?;
        let 输出路径 = 图标目录.join(校验相对路径(&输出, "fonts[].output")?);
        let 许可 = 字符串字段(字体, "license")?;
        let 许可地址 = 字符串字段(字体, "license_url")?;
        let 署名 = 字符串字段(字体, "attribution")?;
        let 字体内缩 = 可选数值(字体.get("weight_inset"))?;
        let 字形条目 = 字体["glyphs"]
            .as_array()
            .ok_or_else(|| 工具错误::新(format!("字体 {id} 缺少 glyphs")))?;
        if 字形条目.is_empty() {
            return Err(工具错误::新(format!("字体 {id} 的 glyphs 不能为空")));
        }
        let mut 名称表 = vec![".notdef".to_string()];
        let mut 轮廓表: Vec<Vec<子路径>> = vec![Vec::new()];
        let mut 码点表: Vec<(u16, usize)> = Vec::new();
        for 条目 in 字形条目 {
            let 文件 = 字符串字段(条目, "file")?;
            let 源路径 = 图标目录.join(校验相对路径(&文件, "glyphs[].file")?);
            crate::文件事务::校验普通路径(&源路径)?;
            let 数据 = std::fs::read(&源路径).map_err(|错误| {
                工具错误::带来源(format!("无法读取 {}", 源路径.display()), 错误)
            })?;
            let 摘要 = sha256十六进制(&数据);
            let 期望 = 字符串字段(条目, "sha256")?;
            if 摘要 != 期望 {
                return Err(工具错误::新(format!(
                    "来源文件已变化：{}；需同步来源记录",
                    Path::new(&文件)
                        .file_name()
                        .map_or_else(String::new, |名| 名.to_string_lossy().to_string())
                )));
            }
            let 文本 = String::from_utf8(数据)
                .map_err(|_| 工具错误::新(format!("{文件} 不是 UTF-8 文本")))?;
            let 文件名 = Path::new(&文件)
                .file_name()
                .map_or_else(String::new, |名| 名.to_string_lossy().to_string());
            let mut 轮廓 = Vec::new();
            let 允许半透明 = match 条目.get("半透明处理").and_then(Value::as_str) {
                None if 条目.get("半透明处理").is_none() => false,
                Some("不透明轮廓") => true,
                _ => return Err(工具错误::新(format!("{文件名} 的半透明处理策略无效"))),
            };
            绘制svg策略(&文本, &文件名, &mut 轮廓, 允许半透明)?;
            if 轮廓.is_empty() {
                return Err(工具错误::新(format!("字形为空：{文件名}")));
            }
            let 内缩 = 可选数值(条目.get("weight_inset"))?
                .or(字体内缩)
                .unwrap_or(0.0);
            if !内缩.is_finite() || !(0.0..=0.25).contains(&内缩) {
                return Err(工具错误::新(format!(
                    "字重内缩需在 0 至 0.25 个 SVG 像素之间：{文件名}"
                )));
            }
            let 轮廓 = if 内缩 > 0.0 {
                应用字重内缩(&轮廓, 内缩)?
            } else {
                轮廓
            };
            let 码点文本 = 字符串字段(条目, "codepoint")?;
            let 码点 = u16::from_str_radix(&码点文本, 16)
                .map_err(|_| 工具错误::新(format!("{文件名} 的码点无效：{码点文本}")))?;
            名称表.push(format!("uni{}", 码点文本.to_uppercase()));
            码点表.push((码点, 轮廓表.len()));
            轮廓表.push(轮廓);
        }
        let ps名 = format!("{}-Regular", 族名.replace(' ', ""));
        let 描述 = 字体描述 {
            族名: &族名,
            样式名: "Regular",
            唯一标识: &ps名,
            全名: &族名,
            ps名: &ps名,
            版本: "Version 1.0",
            版权: &署名,
            许可: &format!("{许可}；由 AdwCode 转换为单色轮廓；来源与字重参数见 来源.json"),
            许可地址: &许可地址,
        };
        操作.push((
            输出路径,
            Some(生成字体字节(&名称表, &轮廓表, &码点表, &描述)?),
        ));
        操作.extend(导出预览(
            &图标目录,
            &id路径,
            字形条目,
            &轮廓表,
            &许可,
            &署名,
        )?);
        let 状态 = if 字体.get("enabled").and_then(Value::as_bool).unwrap_or(true) {
            "启用"
        } else {
            "备用"
        };
        消息.push(format!(
            "已生成 {输出}：{} 个字形，{许可}，{状态}",
            字形条目.len()
        ));
    }
    crate::文件事务::写入批次(&操作)?;
    for 行 in 消息 {
        println!("{行}");
    }
    Ok(())
}

/// 读取字符串字段。
fn 字符串字段(值: &Value, 键: &str) -> 结果<String> {
    值[键]
        .as_str()
        .filter(|文本| !文本.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| 工具错误::新(format!("来源.json 字段缺失或为空：{键}")))
}

/// 校验来源.json 中的相对路径：只允许普通组件，拒绝绝对路径与 `..`。
fn 校验相对路径(文本: &str, 字段: &str) -> 结果<PathBuf> {
    if 文本.is_empty() {
        return Err(工具错误::新(format!("来源.json 的 {字段} 不能为空")));
    }
    let 路径 = Path::new(文本);
    if !路径
        .components()
        .all(|组件| matches!(组件, Component::Normal(_)))
    {
        return Err(工具错误::新(format!(
            "来源.json 的 {字段} 必须是产品图标目录内的相对路径：{文本}"
        )));
    }
    Ok(路径.to_path_buf())
}

/// 读取可选数值字段；存在但不是数值时报错。
fn 可选数值(值: Option<&Value>) -> 结果<Option<f64>> {
    match 值 {
        None | Some(Value::Null) => Ok(None),
        Some(值) => 值
            .as_f64()
            .map(Some)
            .ok_or_else(|| 工具错误::新("来源.json 的字重内缩必须是数值")),
    }
}
