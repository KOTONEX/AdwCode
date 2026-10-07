// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

use super::几何::{
    bezipath转轮廓, flo转轮廓, 子路径, 字体空间轮廓, 按填充规则简化, 简化轮廓, 线段, 规范化朝向,
    轮廓转flo,
};
use super::{SVG命名空间, 坐标上限, 最大嵌套深度};
use crate::错误::{工具错误, 结果};
use flo_curves::bezier::path::{SimpleBezierPath, path_intersect};
use kurbo::{Affine, Arc, Cap, Join, Point, Stroke, StrokeOpts, SvgArc, Vec2};
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use regex::Regex;
use std::collections::BTreeMap;

/// XML 元素节点；属性保留原始限定名，命名空间仅用于判断是否 SVG。
pub(super) struct 节点 {
    本地名: String,
    是svg: bool,
    外部命名空间: bool,
    属性: BTreeMap<String, String>,
    子: Vec<节点>,
}

/// 解析 XML 为元素树；仅保留元素与属性，忽略文本与注释。
pub(super) fn 解析xml(文本: &str, 文件名: &str) -> 结果<节点> {
    let mut 读取器 = Reader::from_str(文本);
    let mut 根: Option<节点> = None;
    let mut 节点栈: Vec<节点> = Vec::new();
    let mut 命名空间栈: Vec<(Option<String>, BTreeMap<String, String>)> =
        vec![(None, BTreeMap::new())];
    loop {
        let 事件 = 读取器
            .read_event()
            .map_err(|错误| 工具错误::新(format!("{文件名} XML 解析失败：{错误}")))?;
        match &事件 {
            Event::Start(元素) | Event::Empty(元素) => {
                if 节点栈.is_empty() && 根.is_some() {
                    return Err(工具错误::新(format!("{文件名} 包含多个 XML 根节点")));
                }
                let 是空元素 = matches!(事件, Event::Empty(_));
                let 原始名 = String::from_utf8_lossy(元素.name().as_ref()).to_string();
                let mut 属性 = BTreeMap::new();
                for 属性项 in 元素.attributes() {
                    let 属性项 = 属性项.map_err(|错误| {
                        工具错误::新(format!("{文件名} XML 属性解析失败：{错误}"))
                    })?;
                    let 键 = String::from_utf8_lossy(属性项.key.as_ref()).to_string();
                    let 值 = 属性项
                        .decoded_and_normalized_value(XmlVersion::default(), 读取器.decoder())
                        .map_err(|错误| {
                            工具错误::新(format!("{文件名} XML 属性解码失败：{错误}"))
                        })?
                        .to_string();
                    属性.insert(键, 值);
                }
                let (默认命名空间, 前缀表) = 命名空间栈.last().cloned().unwrap_or_default();
                let (mut 新默认, mut 新前缀) = (默认命名空间, 前缀表);
                for (键, 值) in &属性 {
                    if 键 == "xmlns" {
                        新默认 = Some(值.clone());
                    } else if let Some(前缀) = 键.strip_prefix("xmlns:") {
                        新前缀.insert(前缀.to_string(), 值.clone());
                    }
                }
                let (前缀名, 本地名) = 原始名
                    .split_once(':')
                    .map_or((None, 原始名.as_str()), |(前缀, 本地)| {
                        (Some(前缀), 本地)
                    });
                let 命名空间 = match 前缀名 {
                    Some(前缀) => 新前缀.get(前缀).cloned(),
                    None => 新默认.clone(),
                };
                let 是svg = 命名空间.as_deref() == Some(SVG命名空间);
                let 外部命名空间 = 命名空间.as_deref().is_some_and(|值| 值 != SVG命名空间);
                属性.retain(|键, _| 键 != "xmlns" && !键.starts_with("xmlns:"));
                let 节点 = 节点 {
                    本地名: 本地名.to_string(),
                    是svg,
                    外部命名空间,
                    属性,
                    子: Vec::new(),
                };
                if 是空元素 {
                    if let Some(父) = 节点栈.last_mut() {
                        父.子.push(节点);
                    } else {
                        根 = Some(节点);
                    }
                } else {
                    if 节点栈.len() >= 最大嵌套深度 {
                        return Err(工具错误::新(format!("{文件名} 的 XML 嵌套过深")));
                    }
                    命名空间栈.push((新默认, 新前缀));
                    节点栈.push(节点);
                }
            }
            Event::End(_) => {
                命名空间栈.pop();
                if let Some(节点) = 节点栈.pop() {
                    if let Some(父) = 节点栈.last_mut() {
                        父.子.push(节点);
                    } else {
                        根 = Some(节点);
                    }
                }
            }
            Event::Eof => {
                if !节点栈.is_empty() {
                    return Err(工具错误::新(format!("{文件名} XML 节点未闭合")));
                }
                break;
            }
            _ => {}
        }
    }
    根.ok_or_else(|| 工具错误::新(format!("{文件名} 不是有效的 XML")))
}

/// 解析 SVG 的 `d` 属性为子路径列表。
pub(super) fn 解析路径数据(数据: &str, 文件名: &str) -> 结果<Vec<子路径>> {
    let mut 子路径列表: Vec<子路径> = Vec::new();
    let mut 当前 = Point::ZERO;
    let mut 起点 = Point::ZERO;
    let mut 上一控制: Option<Point> = None;
    let mut 上一命令 = '\0';
    let mut 当前路径: Option<子路径> = None;
    let mut 有起点 = false;
    for 段 in svgtypes::PathParser::from(数据) {
        let 段 = 段.map_err(|错误| 工具错误::新(format!("{文件名} 路径解析失败：{错误}")))?;
        match 段 {
            svgtypes::PathSegment::MoveTo { abs, x, y } => {
                if let Some(路径) = 当前路径.take() {
                    子路径列表.push(路径);
                }
                let 目标 = 取值(abs, 当前, x, y);
                if !段坐标有效(&线段::直线(目标)) {
                    return Err(工具错误::新(
                        format!("{文件名} 的起点坐标超出允许范围"),
                    ));
                }
                当前 = 目标;
                起点 = 目标;
                有起点 = true;
                当前路径 = Some(子路径 {
                    起点: 目标,
                    段: Vec::new(),
                    闭合: false,
                });
                上一控制 = None;
                上一命令 = 'M';
            }
            svgtypes::PathSegment::LineTo { abs, x, y } => {
                let 目标 = 取值(abs, 当前, x, y);
                推段(&mut 当前路径, 起点, 有起点, 文件名, 线段::直线(目标))?;
                当前 = 目标;
                上一控制 = None;
                上一命令 = 'L';
            }
            svgtypes::PathSegment::HorizontalLineTo { abs, x } => {
                let 目标 = Point::new(if abs { x } else { 当前.x + x }, 当前.y);
                推段(&mut 当前路径, 起点, 有起点, 文件名, 线段::直线(目标))?;
                当前 = 目标;
                上一控制 = None;
                上一命令 = 'L';
            }
            svgtypes::PathSegment::VerticalLineTo { abs, y } => {
                let 目标 = Point::new(当前.x, if abs { y } else { 当前.y + y });
                推段(&mut 当前路径, 起点, 有起点, 文件名, 线段::直线(目标))?;
                当前 = 目标;
                上一控制 = None;
                上一命令 = 'L';
            }
            svgtypes::PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let 控制1 = 取值(abs, 当前, x1, y1);
                let 控制2 = 取值(abs, 当前, x2, y2);
                let 目标 = 取值(abs, 当前, x, y);
                推段(
                    &mut 当前路径,
                    起点,
                    有起点,
                    文件名,
                    线段::三次(控制1, 控制2, 目标),
                )?;
                当前 = 目标;
                上一控制 = Some(控制2);
                上一命令 = 'C';
            }
            svgtypes::PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                let 控制1 = if matches!(上一命令, 'C' | 'S') {
                    上一控制.map_or(当前, |控制| {
                        Point::new(当前.x * 2.0 - 控制.x, 当前.y * 2.0 - 控制.y)
                    })
                } else {
                    当前
                };
                let 控制2 = 取值(abs, 当前, x2, y2);
                let 目标 = 取值(abs, 当前, x, y);
                推段(
                    &mut 当前路径,
                    起点,
                    有起点,
                    文件名,
                    线段::三次(控制1, 控制2, 目标),
                )?;
                当前 = 目标;
                上一控制 = Some(控制2);
                上一命令 = 'S';
            }
            svgtypes::PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let 控制 = 取值(abs, 当前, x1, y1);
                let 目标 = 取值(abs, 当前, x, y);
                推段(&mut 当前路径, 起点, 有起点, 文件名, 线段::二次(控制, 目标))?;
                当前 = 目标;
                上一控制 = Some(控制);
                上一命令 = 'Q';
            }
            svgtypes::PathSegment::SmoothQuadratic { abs, x, y } => {
                let 控制 = if matches!(上一命令, 'Q' | 'T') {
                    上一控制.map_or(当前, |控制| {
                        Point::new(当前.x * 2.0 - 控制.x, 当前.y * 2.0 - 控制.y)
                    })
                } else {
                    当前
                };
                let 目标 = 取值(abs, 当前, x, y);
                推段(&mut 当前路径, 起点, 有起点, 文件名, 线段::二次(控制, 目标))?;
                当前 = 目标;
                上一控制 = Some(控制);
                上一命令 = 'T';
            }
            svgtypes::PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let 目标 = 取值(abs, 当前, x, y);
                if [rx, ry, x_axis_rotation]
                    .iter()
                    .any(|值| !值.is_finite() || 值.abs() > 坐标上限)
                    || rx < 0.0
                    || ry < 0.0
                {
                    return Err(工具错误::新(format!("{文件名} 的圆弧参数越界")));
                }
                if !段坐标有效(&线段::直线(目标)) {
                    return Err(工具错误::新(format!("{文件名} 的圆弧终点越界")));
                }
                let 圆弧 = SvgArc {
                    from: 当前,
                    to: 目标,
                    radii: Vec2::new(rx, ry),
                    x_rotation: x_axis_rotation.to_radians(),
                    large_arc,
                    sweep,
                };
                if let Some(弧) = Arc::from_svg_arc(&圆弧) {
                    let mut 段列表 = Vec::new();
                    弧.to_cubic_beziers(0.05, |控制1, 控制2, 终点| {
                        段列表.push(线段::三次(控制1, 控制2, 终点));
                    });
                    for 段项 in 段列表 {
                        推段(&mut 当前路径, 起点, 有起点, 文件名, 段项)?;
                        当前 = 段项.终点();
                    }
                } else {
                    推段(&mut 当前路径, 起点, 有起点, 文件名, 线段::直线(目标))?;
                    当前 = 目标;
                }
                上一控制 = None;
                上一命令 = 'A';
            }
            svgtypes::PathSegment::ClosePath { .. } => {
                if let Some(mut 路径) = 当前路径.take() {
                    路径.闭合 = true;
                    子路径列表.push(路径);
                }
                当前 = 起点;
                上一控制 = None;
                上一命令 = 'Z';
            }
        }
    }
    if let Some(路径) = 当前路径.take() {
        子路径列表.push(路径);
    }
    Ok(子路径列表)
}

/// 相对/绝对坐标换算。
pub(super) fn 取值(abs: bool, 当前: Point, x: f64, y: f64) -> Point {
    if abs {
        Point::new(x, y)
    } else {
        Point::new(当前.x + x, 当前.y + y)
    }
}

/// 追加线段；缺少 MoveTo 时报错，`Z` 之后的绘制命令会从起点另起子路径。
pub(super) fn 推段(
    当前路径: &mut Option<子路径>,
    缺省起点: Point,
    有起点: bool,
    文件名: &str,
    段: 线段,
) -> 结果<()> {
    if !有起点 {
        return Err(工具错误::新(
            format!("{文件名} 路径数据缺少 M 起始命令"),
        ));
    }
    if !段坐标有效(&段) {
        return Err(工具错误::新(format!(
            "{文件名} 路径数据包含非有限或超幅坐标"
        )));
    }
    let 路径 = 当前路径.get_or_insert_with(|| 子路径 {
        起点: 缺省起点,
        段: Vec::new(),
        闭合: false,
    });
    路径.段.push(段);
    Ok(())
}

/// 线段所有坐标都有限且不超过坐标上限。
pub(super) fn 段坐标有效(段: &线段) -> bool {
    let 有效 = |点: &Point| {
        点.x.is_finite() && 点.y.is_finite() && 点.x.abs() <= 坐标上限 && 点.y.abs() <= 坐标上限
    };
    match *段 {
        线段::直线(终点) => 有效(&终点),
        线段::二次(控制, 终点) => 有效(&控制) && 有效(&终点),
        线段::三次(控制1, 控制2, 终点) => 有效(&控制1) && 有效(&控制2) && 有效(&终点),
    }
}

/// 在轮廓布尔算法之前拒绝变换产生的非法坐标。
pub(super) fn 校验变换轮廓(轮廓: &子路径, 文件名: &str) -> 结果<()> {
    if !段坐标有效(&线段::直线(轮廓.起点)) || !轮廓.段.iter().all(段坐标有效) {
        return Err(工具错误::新(format!(
            "{文件名} 的变换轮廓包含非有限或超幅坐标"
        )));
    }
    Ok(())
}

/// 解析元素变换；只接受 matrix/translate/scale，其余明确报错。
pub(super) fn 解析变换(值: &str, 文件名: &str) -> 结果<Affine> {
    if 值.trim().is_empty() {
        return Ok(Affine::IDENTITY);
    }
    static 变换正则: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(matrix|translate|scale)\s*\(([^)]+)\)").expect("变换正则")
    });
    let mut 结果 = Affine::IDENTITY;
    let mut 剩余 = 值.to_string();
    for 匹配 in 变换正则.find_iter(值) {
        let 捕获 = 变换正则.captures(匹配.as_str()).expect("变换捕获");
        let 名称 = 捕获.get(1).expect("变换名").as_str();
        let 参数文本 = 捕获.get(2).expect("变换参数").as_str().trim();
        let 参数 = 解析数值列表(参数文本, 文件名, " SVG 变换")?;
        let 当前 = match 名称 {
            "matrix" if 参数.len() == 6 => {
                Affine::new([参数[0], 参数[1], 参数[2], 参数[3], 参数[4], 参数[5]])
            }
            "translate" if 参数.len() == 1 => Affine::translate((参数[0], 0.0)),
            "translate" if 参数.len() == 2 => Affine::translate((参数[0], 参数[1])),
            "scale" if 参数.len() == 1 => Affine::scale(参数[0]),
            "scale" if 参数.len() == 2 => Affine::scale_non_uniform(参数[0], 参数[1]),
            _ => {
                return Err(工具错误::新(format!(
                    "{文件名} 不支持的 SVG 变换：{}",
                    匹配.as_str()
                )));
            }
        };
        结果 *= 当前;
        剩余 = 剩余.replacen(匹配.as_str(), "", 1);
    }
    if !剩余.trim().is_empty() {
        return Err(工具错误::新(
            format!("{文件名} 不支持的 SVG 变换：{值}"),
        ));
    }
    Ok(结果)
}

/// 解析 `style` 属性为键值表。
pub(super) fn 解析style(值: Option<&String>) -> BTreeMap<String, String> {
    let mut 结果 = BTreeMap::new();
    if let Some(文本) = 值 {
        for 项 in 文本.split(';') {
            if let Some((键, 值)) = 项.split_once(':') {
                结果.insert(键.trim().to_string(), 值.trim().to_string());
            }
        }
    }
    结果
}

/// 合并元素属性与 style（style 覆盖同名属性）。
pub(super) fn 合并属性(节点: &节点) -> BTreeMap<String, String> {
    let mut 结果 = 节点.属性.clone();
    for (键, 值) in 解析style(节点.属性.get("style")) {
        结果.insert(键, 值);
    }
    结果
}

/// 读取数值属性，缺省时使用默认值；拒绝非有限或超幅值。
pub(super) fn 数值属性(
    属性: &BTreeMap<String, String>,
    键: &str,
    默认: f64,
    文件名: &str,
) -> 结果<f64> {
    match 属性.get(键) {
        Some(文本) => {
            let 值 = 文本
                .trim()
                .parse::<f64>()
                .map_err(|_| 工具错误::新(format!("{文件名} 无法解析 {键}：{文本}")))?;
            if !值.is_finite() || 值.abs() > 坐标上限 {
                return Err(工具错误::新(format!(
                    "{文件名} 的 {键} 超出允许范围：{文本}"
                )));
            }
            Ok(值)
        }
        None => Ok(默认),
    }
}

/// 解析以空白或逗号分隔的数值列表；拒绝非有限或超幅值。
pub(super) fn 解析数值列表(文本: &str, 文件名: &str, 说明: &str) -> 结果<Vec<f64>> {
    let mut 结果 = Vec::new();
    for 项 in 文本.split([' ', ',', '\t', '\n', '\r']) {
        if 项.is_empty() {
            continue;
        }
        let 值 = 项
            .parse::<f64>()
            .map_err(|_| 工具错误::新(format!("{文件名} 无法解析{说明}：{文本}")))?;
        if !值.is_finite() || 值.abs() > 坐标上限 {
            return Err(工具错误::新(format!(
                "{文件名} 的{说明}超出允许范围：{文本}"
            )));
        }
        结果.push(值);
    }
    Ok(结果)
}

/// 元素对应的一条或多条子路径；不支持的标签返回错误。
pub(super) fn 元素子路径(节点: &节点, 文件名: &str) -> 结果<Vec<子路径>> {
    match 节点.本地名.as_str() {
        "path" => {
            let 数据 = 节点
                .属性
                .get("d")
                .ok_or_else(|| 工具错误::新(format!("{文件名} 的 path 缺少 d 属性")))?;
            解析路径数据(数据, 文件名)
        }
        "rect" => {
            let 属性 = &节点.属性;
            let x = 数值属性(属性, "x", 0.0, 文件名)?;
            let y = 数值属性(属性, "y", 0.0, 文件名)?;
            let 宽 = 数值属性(属性, "width", 0.0, 文件名)?;
            let 高 = 数值属性(属性, "height", 0.0, 文件名)?;
            if 宽 <= 0.0 || 高 <= 0.0 {
                return Ok(Vec::new());
            }
            let rx = 数值属性(属性, "rx", 0.0, 文件名)?;
            let ry = 数值属性(属性, "ry", 0.0, 文件名)?;
            let (rx, ry) = match (rx > 0.0, ry > 0.0) {
                (true, true) => (rx.min(宽 / 2.0), ry.min(高 / 2.0)),
                (true, false) => (rx.min(宽 / 2.0), rx.min(高 / 2.0)),
                (false, true) => (ry.min(宽 / 2.0), ry.min(高 / 2.0)),
                (false, false) => (0.0, 0.0),
            };
            Ok(矩形路径(x, y, x + 宽, y + 高, rx, ry))
        }
        "circle" => {
            let cx = 数值属性(&节点.属性, "cx", 0.0, 文件名)?;
            let cy = 数值属性(&节点.属性, "cy", 0.0, 文件名)?;
            let r = 数值属性(&节点.属性, "r", 0.0, 文件名)?;
            Ok(椭圆路径(cx, cy, r, r))
        }
        "ellipse" => {
            let cx = 数值属性(&节点.属性, "cx", 0.0, 文件名)?;
            let cy = 数值属性(&节点.属性, "cy", 0.0, 文件名)?;
            let rx = 数值属性(&节点.属性, "rx", 0.0, 文件名)?;
            let ry = 数值属性(&节点.属性, "ry", 0.0, 文件名)?;
            Ok(椭圆路径(cx, cy, rx, ry))
        }
        "line" => {
            let x1 = 数值属性(&节点.属性, "x1", 0.0, 文件名)?;
            let y1 = 数值属性(&节点.属性, "y1", 0.0, 文件名)?;
            let x2 = 数值属性(&节点.属性, "x2", 0.0, 文件名)?;
            let y2 = 数值属性(&节点.属性, "y2", 0.0, 文件名)?;
            Ok(vec![子路径 {
                起点: Point::new(x1, y1),
                段: vec![线段::直线(Point::new(x2, y2))],
                闭合: false,
            }])
        }
        "polyline" | "polygon" => {
            let 文本 = 节点.属性.get("points").ok_or_else(|| {
                工具错误::新(format!("{文件名} 的 {} 缺少 points", 节点.本地名))
            })?;
            let 点列 = 解析点列(文本, 文件名)?;
            if 点列.len() < 2 {
                return Ok(Vec::new());
            }
            let mut 段: Vec<线段> = 点列[1..].iter().map(|点| 线段::直线(*点)).collect();
            let 闭合 = 节点.本地名 == "polygon";
            if 闭合 && 点列.last() != Some(&点列[0]) {
                段.push(线段::直线(点列[0]));
            }
            Ok(vec![子路径 {
                起点: 点列[0],
                段,
                闭合,
            }])
        }
        其他 => Err(工具错误::新(
            format!("{文件名} 存在不支持的元素：{其他}"),
        )),
    }
}

/// 解析 points 属性为点列。
pub(super) fn 解析点列(文本: &str, 文件名: &str) -> 结果<Vec<Point>> {
    let 数值 = 解析数值列表(文本, 文件名, " points")?;
    if 数值.len() % 2 != 0 {
        return Err(工具错误::新(format!("{文件名} 的 points 坐标不成对")));
    }
    Ok(数值
        .as_chunks::<2>()
        .0
        .iter()
        .map(|对| Point::new(对[0], 对[1]))
        .collect())
}

/// 矩形路径；圆角半径为零时为四条直线。
pub(super) fn 矩形路径(x0: f64, y0: f64, x1: f64, y1: f64, rx: f64, ry: f64) -> Vec<子路径> {
    if rx <= 0.0 || ry <= 0.0 {
        return vec![子路径 {
            起点: Point::new(x0, y0),
            段: vec![
                线段::直线(Point::new(x1, y0)),
                线段::直线(Point::new(x1, y1)),
                线段::直线(Point::new(x0, y1)),
            ],
            闭合: true,
        }];
    }
    const 圆角系数: f64 = 0.552_284_749_830_793_4;
    let (控制x, 控制y) = (rx * 圆角系数, ry * 圆角系数);
    vec![子路径 {
        起点: Point::new(x0 + rx, y0),
        段: vec![
            线段::直线(Point::new(x1 - rx, y0)),
            线段::三次(
                Point::new(x1 - rx + 控制x, y0),
                Point::new(x1, y0 + ry - 控制y),
                Point::new(x1, y0 + ry),
            ),
            线段::直线(Point::new(x1, y1 - ry)),
            线段::三次(
                Point::new(x1, y1 - ry + 控制y),
                Point::new(x1 - rx + 控制x, y1),
                Point::new(x1 - rx, y1),
            ),
            线段::直线(Point::new(x0 + rx, y1)),
            线段::三次(
                Point::new(x0 + rx - 控制x, y1),
                Point::new(x0, y1 - ry + 控制y),
                Point::new(x0, y1 - ry),
            ),
            线段::直线(Point::new(x0, y0 + ry)),
            线段::三次(
                Point::new(x0, y0 + ry - 控制y),
                Point::new(x0 + rx - 控制x, y0),
                Point::new(x0 + rx, y0),
            ),
        ],
        闭合: true,
    }]
}

/// 椭圆路径（四段三次曲线）。
pub(super) fn 椭圆路径(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<子路径> {
    if rx <= 0.0 || ry <= 0.0 {
        return Vec::new();
    }
    const 圆角系数: f64 = 0.552_284_749_830_793_4;
    let (左, 右) = (cx - rx, cx + rx);
    let (上, 下) = (cy - ry, cy + ry);
    let 控制x = rx * 圆角系数;
    let 控制y = ry * 圆角系数;
    vec![子路径 {
        起点: Point::new(cx, 上),
        段: vec![
            线段::三次(
                Point::new(cx + 控制x, 上),
                Point::new(右, cy - 控制y),
                Point::new(右, cy),
            ),
            线段::三次(
                Point::new(右, cy + 控制y),
                Point::new(cx + 控制x, 下),
                Point::new(cx, 下),
            ),
            线段::三次(
                Point::new(cx - 控制x, 下),
                Point::new(左, cy + 控制y),
                Point::new(左, cy),
            ),
            线段::三次(
                Point::new(左, cy - 控制y),
                Point::new(cx - 控制x, 上),
                Point::new(cx, 上),
            ),
        ],
        闭合: true,
    }]
}

/// 视口矩形，用于描边轮廓的裁切。
pub(super) fn 视口路径() -> 子路径 {
    子路径 {
        起点: Point::new(0.0, 0.0),
        段: vec![
            线段::直线(Point::new(16.0, 0.0)),
            线段::直线(Point::new(16.0, 16.0)),
            线段::直线(Point::new(0.0, 16.0)),
        ],
        闭合: true,
    }
}

/// 逐轮廓处理：跳过画布外、拒绝可见特效与超界，再转字体空间。
pub(super) fn 处理轮廓(
    轮廓: 子路径,
    特效: bool,
    文件名: &str,
    收集: &mut Vec<子路径>,
) -> 结果<()> {
    let Some((左, 上, 右, 下)) = 轮廓.包围盒() else {
        return Ok(());
    };
    if 右 <= 0.0 || 下 <= 0.0 || 左 >= 16.0 || 上 >= 16.0 {
        return Ok(());
    }
    if 特效 {
        return Err(工具错误::新(format!(
            "{文件名} 存在可见遮罩、裁切或半透明效果，不能直接转换为轮廓字体"
        )));
    }
    if 左.min(上) < -0.0625 || 右.max(下) > 16.0625 {
        return Err(工具错误::新(format!("{文件名} 的可见轮廓超出画布")));
    }
    收集.push(字体空间轮廓(&轮廓));
    Ok(())
}

/// 访问 SVG 元素树，按 Python 版规则收集可见轮廓。
pub(super) fn 访问元素(
    节点: &节点,
    变换: Affine,
    特效: bool,
    继承: &BTreeMap<String, String>,
    允许半透明: bool,
    文件名: &str,
    收集: &mut Vec<子路径>,
) -> 结果<()> {
    if 节点.外部命名空间 {
        return Ok(());
    }
    if matches!(
        节点.本地名.as_str(),
        "defs" | "filter" | "mask" | "clipPath" | "metadata" | "title" | "desc"
    ) {
        return Ok(());
    }
    let mut 属性 = 合并属性(节点);
    for 键 in [
        "fill",
        "stroke",
        "fill-rule",
        "fill-opacity",
        "stroke-opacity",
        "stroke-width",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-dasharray",
        "stroke-dashoffset",
        "visibility",
        "color",
    ] {
        if 属性
            .get(键)
            .is_none_or(|值| 值 == "inherit" || 值 == "unset")
        {
            if let Some(值) = 继承.get(键) {
                属性.insert(键.to_string(), 值.clone());
            } else {
                属性.remove(键);
            }
        }
    }
    if 属性.get("display").is_some_and(|值| 值 == "none")
        || 透明度属性(&属性, "opacity", 文件名)? == 0.0
    {
        return Ok(());
    }
    let 自身 = 解析变换(
        节点.属性.get("transform").map_or("", String::as_str),
        文件名,
    )?;
    let 变换 = 变换 * 自身;
    if 变换
        .as_coeffs()
        .iter()
        .any(|值| !值.is_finite() || 值.abs() > 坐标上限)
    {
        return Err(工具错误::新(format!(
            "{文件名} 的组合变换包含非有限或超幅系数"
        )));
    }
    let 特效 = 特效
        || ["mask", "clip-path", "filter"]
            .iter()
            .any(|键| 属性.get(*键).is_some_and(|值| 值 != "none"))
        || (!允许半透明 && 透明度属性(&属性, "opacity", 文件名)? != 1.0);
    if 节点.本地名 == "svg" || 节点.本地名 == "g" {
        for 子 in &节点.子 {
            访问元素(子, 变换, 特效, &属性, 允许半透明, 文件名, 收集)?;
        }
        return Ok(());
    }
    if 属性
        .get("visibility")
        .is_some_and(|值| 值 == "hidden" || 值 == "collapse")
    {
        return Ok(());
    }
    for (键, 透明度键) in [("fill", "fill-opacity"), ("stroke", "stroke-opacity")] {
        if 透明度属性(&属性, 透明度键, 文件名)? == 0.0
            || 属性.get(键).is_some_and(|值| 值 == "transparent")
        {
            属性.insert(键.to_string(), "none".to_string());
        }
    }
    let 子路径列表 = 元素子路径(节点, 文件名)?;
    let 特效 = 特效
        || [("fill", "fill-opacity"), ("stroke", "stroke-opacity")]
            .iter()
            .any(|(键, 透明度键)| {
                属性.get(*键).map_or(*键 == "fill", |值| 值 != "none")
                    && ((!允许半透明
                        && 透明度属性(&属性, 透明度键, 文件名).is_ok_and(|值| 值 != 1.0))
                        || 属性.get(*键).is_some_and(|值| {
                            值.contains("url(") || (!允许半透明 && 调色板透明(值))
                        }))
            });
    let 描边 = 属性.get("stroke").is_some_and(|值| 值 != "none");
    if 属性.get("fill").is_some_and(|值| 值 == "none") && !描边 {
        return Ok(());
    }
    if 描边 && 属性.get("fill").is_none_or(|值| 值 != "none") {
        return Err(工具错误::新(format!(
            "{文件名} 同时包含填充与描边，需要单独转换"
        )));
    }
    if 描边 {
        if 属性.contains_key("stroke-dasharray") || 属性.contains_key("stroke-dashoffset") {
            return Err(工具错误::新(
                format!("{文件名} 使用尚未支持的虚线描边"),
            ));
        }
        let 宽度 = 数值属性(&属性, "stroke-width", 1.0, 文件名)?;
        let 端点 = match 属性.get("stroke-linecap").map_or("butt", String::as_str) {
            "butt" => Cap::Butt,
            "round" => Cap::Round,
            "square" => Cap::Square,
            其他 => {
                return Err(工具错误::新(
                    format!("{文件名} 不支持的描边端点：{其他}"),
                ));
            }
        };
        let 连接 = match 属性.get("stroke-linejoin").map_or("miter", String::as_str) {
            "miter" => Join::Miter,
            "round" => Join::Round,
            "bevel" => Join::Bevel,
            其他 => {
                return Err(工具错误::新(
                    format!("{文件名} 不支持的描边连接：{其他}"),
                ));
            }
        };
        let 样式 = Stroke {
            join: 连接,
            start_cap: 端点,
            end_cap: 端点,
            miter_limit: 数值属性(&属性, "stroke-miterlimit", 4.0, 文件名)?,
            ..Stroke::new(宽度)
        };
        let mut 描边轮廓: Vec<子路径> = Vec::new();
        for 子 in &子路径列表 {
            let 结果 = kurbo::stroke(子.描边bezipath(), &样式, &StrokeOpts::default(), 0.01);
            描边轮廓.extend(bezipath转轮廓(&结果));
        }
        let 描边轮廓: Vec<子路径> = 描边轮廓.iter().map(|项| 项.变换(变换)).collect();
        for 轮廓 in &描边轮廓 {
            校验变换轮廓(轮廓, 文件名)?;
        }
        let 简化 = 简化轮廓(&描边轮廓, 0.01);
        let 简化flo: Vec<SimpleBezierPath> = 简化.iter().map(轮廓转flo).collect();
        let 视口flo = 轮廓转flo(&视口路径());
        let 可见: Vec<子路径> = path_intersect(&简化flo, &vec![视口flo], 0.01)
            .iter()
            .map(flo转轮廓)
            .collect();
        for 轮廓 in 规范化朝向(&可见) {
            处理轮廓(轮廓, 特效, 文件名, 收集)?;
        }
        return Ok(());
    }
    let 奇偶 = match 属性.get("fill-rule").map_or("nonzero", String::as_str) {
        "nonzero" => false,
        "evenodd" => true,
        其他 => {
            return Err(工具错误::新(
                format!("{文件名} 的 fill-rule 无效：{其他}"),
            ));
        }
    };
    let mut 可见 = Vec::new();
    for 子 in 子路径列表 {
        let mut 子 = 子.变换(变换);
        校验变换轮廓(&子, 文件名)?;
        子.隐式闭合();
        if 奇偶 {
            if 子
                .包围盒()
                .is_some_and(|(左, 上, 右, 下)| 右 > 0.0 && 下 > 0.0 && 左 < 16.0 && 上 < 16.0)
            {
                可见.push(子);
            }
        } else {
            处理轮廓(子, 特效, 文件名, 收集)?;
        }
    }
    if 奇偶 {
        for 子 in 按填充规则简化(&可见, 0.01, true) {
            处理轮廓(子, 特效, 文件名, 收集)?;
        }
    }
    Ok(())
}

pub(super) fn 调色板透明(文本: &str) -> bool {
    crate::调色板::解析颜色(文本).is_ok_and(|(_, _, _, 透明度)| 透明度 < 1.0)
}

pub(super) fn 透明度属性(
    属性: &BTreeMap<String, String>,
    键: &str,
    文件名: &str,
) -> 结果<f64> {
    let Some(文本) = 属性.get(键) else {
        return Ok(1.0);
    };
    let 值 = 文本
        .strip_suffix('%')
        .unwrap_or(文本)
        .parse::<f64>()
        .ok()
        .map(|值| {
            if 文本.ends_with('%') {
                值 / 100.0
            } else {
                值
            }
        });
    值.filter(|值| 值.is_finite() && (0.0..=1.0).contains(值))
        .ok_or_else(|| 工具错误::新(format!("{文件名} 的 {键} 无效")))
}

/// 读取 SVG 并校验画布，收集可见轮廓（SVG 用户坐标）。
#[cfg(test)]
pub(super) fn 绘制svg(文本: &str, 文件名: &str, 收集: &mut Vec<子路径>) -> 结果<()> {
    绘制svg策略(文本, 文件名, 收集, false)
}

pub(super) fn 绘制svg策略(
    文本: &str,
    文件名: &str,
    收集: &mut Vec<子路径>,
    允许半透明: bool,
) -> 结果<()> {
    let 根 = 解析xml(文本, 文件名)?;
    if 根.本地名 != "svg" || 根.外部命名空间 {
        return Err(工具错误::新(format!("{文件名} 的根元素必须是 SVG")));
    }
    let 画布文本 = 根.属性.get("viewBox").map_or("0 0 16 16", String::as_str);
    let 画布 = 解析数值列表(画布文本, 文件名, " viewBox")?;
    if 画布.as_slice() != [0.0, 0.0, 16.0, 16.0] {
        return Err(工具错误::新(format!("{文件名} 需要 16 × 16 画布")));
    }
    访问元素(
        &根,
        Affine::IDENTITY,
        false,
        &BTreeMap::new(),
        允许半透明,
        文件名,
        收集,
    )
}

/// 递归收集 SVG 命名空间下所有 path 元素（自有字形按整棵树取用）。
pub(super) fn 收集path元素(
    节点: &节点, 文件名: &str, 收集: &mut Vec<子路径>
) -> 结果<()> {
    if 节点.是svg && 节点.本地名 == "path" {
        let 数据 = 节点
            .属性
            .get("d")
            .ok_or_else(|| 工具错误::新(format!("{文件名} 的 path 缺少 d 属性")))?;
        for 子 in 解析路径数据(数据, 文件名)? {
            收集.push(字体空间轮廓(&子));
        }
    }
    for 子 in &节点.子 {
        收集path元素(子, 文件名, 收集)?;
    }
    Ok(())
}
