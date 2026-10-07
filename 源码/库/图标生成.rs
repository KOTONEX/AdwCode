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

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use flo_curves::bezier::path::{
    GraphPath, GraphPathEdgeKind, PathLabel, SimpleBezierPath, path_intersect, path_sub,
};
use flo_curves::bezier::{BezierCurve, NormalCurve};
use flo_curves::{Coord2, Coordinate2D};
use kurbo::{
    Affine, Arc, BezPath, Cap, CubicBez, Join, ParamCurve, ParamCurveDeriv, PathEl, Point, QuadBez,
    Shape, Stroke, StrokeOpts, SvgArc, Vec2,
};
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use regex::Regex;
use serde_json::Value;
use write_fonts::FontBuilder;
use write_fonts::tables::cmap::{Cmap, CmapSubtable, EncodingRecord, PlatformId};
use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::head::{Flags, Head, MacStyle};
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::{Hmtx, LongMetric};
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::name::{Name, NameRecord};
use write_fonts::tables::os2::{Os2, SelectionFlags};
use write_fonts::tables::post::Post;
use write_fonts::types::{Fixed, LongDateTime, NameId, Tag};

use crate::摘要::sha256十六进制;
use crate::错误::{工具错误, 结果};

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

/// 一段轮廓线段；坐标位于所在坐标系。
#[derive(Clone, Copy, Debug)]
enum 线段 {
    直线(Point),
    二次(Point, Point),
    三次(Point, Point, Point),
}

impl 线段 {
    /// 线段终点。
    fn 终点(&self) -> Point {
        match *self {
            线段::直线(终点) | 线段::二次(_, 终点) | 线段::三次(_, _, 终点) => {
                终点
            }
        }
    }
}

/// 一条轮廓：起点加有序线段，闭合标志表示 SVG 的 `Z`。
#[derive(Clone, Debug)]
struct 子路径 {
    起点: Point,
    段: Vec<线段>,
    闭合: bool,
}

impl 子路径 {
    /// 轮廓最后一段的终点；空轮廓返回起点。
    fn 终点(&self) -> Point {
        self.段.last().map_or(self.起点, 线段::终点)
    }

    /// 若未显式闭合，补一条回到起点的直线（字形填充按闭合处理）。
    fn 隐式闭合(&mut self) {
        if !self.闭合 && self.终点() != self.起点 {
            self.段.push(线段::直线(self.起点));
        }
        self.闭合 = true;
    }

    /// 应用仿射变换。
    fn 变换(&self, 变换: Affine) -> Self {
        Self {
            起点: 变换 * self.起点,
            段: self
                .段
                .iter()
                .map(|段| match *段 {
                    线段::直线(终点) => 线段::直线(变换 * 终点),
                    线段::二次(控制, 终点) => 线段::二次(变换 * 控制, 变换 * 终点),
                    线段::三次(控制1, 控制2, 终点) => {
                        线段::三次(变换 * 控制1, 变换 * 控制2, 变换 * 终点)
                    }
                })
                .collect(),
            闭合: self.闭合,
        }
    }

    /// 反转方向（起点保持在原终点）。
    fn 反转(&self) -> Self {
        let mut 段 = Vec::with_capacity(self.段.len());
        for 序号 in (0..self.段.len()).rev() {
            let 段起点 = if 序号 == 0 {
                self.起点
            } else {
                self.段[序号 - 1].终点()
            };
            段.push(match self.段[序号] {
                线段::直线(_) => 线段::直线(段起点),
                线段::二次(控制, _) => 线段::二次(控制, 段起点),
                线段::三次(控制1, 控制2, _) => 线段::三次(控制2, 控制1, 段起点),
            });
        }
        Self {
            起点: self.终点(),
            段,
            闭合: self.闭合,
        }
    }

    /// 转为 kurbo 路径（每条轮廓以 `ClosePath` 结束）。
    fn 转bezipath(&self) -> BezPath {
        let mut 路径 = BezPath::new();
        路径.move_to(self.起点);
        for 段 in &self.段 {
            match *段 {
                线段::直线(终点) => 路径.line_to(终点),
                线段::二次(控制, 终点) => 路径.quad_to(控制, 终点),
                线段::三次(控制1, 控制2, 终点) => 路径.curve_to(控制1, 控制2, 终点),
            }
        }
        路径.close_path();
        路径
    }

    /// 转为 kurbo 路径用于描边；开放轮廓不补 `ClosePath`。
    fn 描边bezipath(&self) -> BezPath {
        let mut 路径 = BezPath::new();
        路径.move_to(self.起点);
        for 段 in &self.段 {
            match *段 {
                线段::直线(终点) => 路径.line_to(终点),
                线段::二次(控制, 终点) => 路径.quad_to(控制, 终点),
                线段::三次(控制1, 控制2, 终点) => 路径.curve_to(控制1, 控制2, 终点),
            }
        }
        if self.闭合 {
            路径.close_path();
        }
        路径
    }

    /// 紧致包围盒（含曲线极值），与 fontTools BoundsPen 一致。
    fn 包围盒(&self) -> Option<(f64, f64, f64, f64)> {
        if self.段.is_empty() {
            return None;
        }
        let 矩形 = self.转bezipath().bounding_box();
        Some((矩形.x0, 矩形.y0, 矩形.x1, 矩形.y1))
    }

    /// 有向面积；正为逆时针（y 向上）。
    fn 有向面积(&self) -> f64 {
        self.转bezipath().area()
    }

    /// 自适应展平为多边形点列（起点开始，不含重复终点）。
    fn 展平(&self, 容差: f64) -> Vec<Point> {
        let mut 点 = vec![self.起点];
        for 段 in &self.段 {
            let 起点 = *点.last().expect("展平点列非空");
            match *段 {
                线段::直线(终点) => 点.push(终点),
                线段::二次(控制, 终点) => 展平二次(起点, 控制, 终点, 容差, 0, &mut 点),
                线段::三次(控制1, 控制2, 终点) => {
                    展平三次(起点, 控制1, 控制2, 终点, 容差, 0, &mut 点);
                }
            }
        }
        点
    }

    /// 轮廓内部的一点：从首段中点沿向内法线偏移，用于包含关系判断。
    fn 内部采样点(&self) -> Point {
        let 向内 = if self.有向面积() > 0.0 { 1.0 } else { -1.0 };
        let mut 起点 = self.起点;
        for 段 in &self.段 {
            let (中点, 切线) = match *段 {
                线段::直线(终点) => (起点.midpoint(终点), 终点 - 起点),
                线段::二次(控制, 终点) => {
                    let 曲线 = QuadBez::new(起点, 控制, 终点);
                    (曲线.eval(0.5), 曲线.deriv().eval(0.5).to_vec2())
                }
                线段::三次(控制1, 控制2, 终点) => {
                    let 曲线 = CubicBez::new(起点, 控制1, 控制2, 终点);
                    (曲线.eval(0.5), 曲线.deriv().eval(0.5).to_vec2())
                }
            };
            if 切线.hypot() > 1e-9 {
                let 法线 = Vec2::new(-切线.y, 切线.x) / 切线.hypot();
                return 中点 + 法线 * 向内 * 0.05;
            }
            起点 = 段.终点();
        }
        self.起点
    }

    /// 导出 SVG 路径命令（`M`/`L`/`Q`/`C`/`Z`）。
    fn svg命令(&self) -> String {
        let mut 文本 = format!("M{} {}", 格式化数值(self.起点.x), 格式化数值(self.起点.y));
        for 段 in &self.段 {
            match *段 {
                线段::直线(终点) => {
                    文本.push_str(&format!("L{} {}", 格式化数值(终点.x), 格式化数值(终点.y)));
                }
                线段::二次(控制, 终点) => {
                    文本.push_str(&format!(
                        "Q{} {} {} {}",
                        格式化数值(控制.x),
                        格式化数值(控制.y),
                        格式化数值(终点.x),
                        格式化数值(终点.y)
                    ));
                }
                线段::三次(控制1, 控制2, 终点) => {
                    文本.push_str(&format!(
                        "C{} {} {} {} {} {}",
                        格式化数值(控制1.x),
                        格式化数值(控制1.y),
                        格式化数值(控制2.x),
                        格式化数值(控制2.y),
                        格式化数值(终点.x),
                        格式化数值(终点.y)
                    ));
                }
            }
        }
        文本.push('Z');
        文本
    }
}

/// 按弦偏差递归展平二次曲线。
fn 展平二次(
    起点: Point,
    控制: Point,
    终点: Point,
    容差: f64,
    深度: u32,
    输出: &mut Vec<Point>,
) {
    if 深度 >= 展平最大深度 {
        输出.push(终点);
        return;
    }
    let 弦 = 终点 - 起点;
    let 偏差 = if 弦.hypot() < 1e-12 {
        (控制 - 起点).hypot()
    } else {
        (控制 - 起点).cross(弦).abs() / 弦.hypot()
    };
    if 偏差 <= 容差 {
        输出.push(终点);
        return;
    }
    let 中点 = 起点.midpoint(控制);
    let 中点2 = 控制.midpoint(终点);
    let 分割点 = 中点.midpoint(中点2);
    展平二次(起点, 中点, 分割点, 容差, 深度 + 1, 输出);
    展平二次(分割点, 中点2, 终点, 容差, 深度 + 1, 输出);
}

/// 按弦偏差递归展平三次曲线。
fn 展平三次(
    起点: Point,
    控制1: Point,
    控制2: Point,
    终点: Point,
    容差: f64,
    深度: u32,
    输出: &mut Vec<Point>,
) {
    if 深度 >= 展平最大深度 {
        输出.push(终点);
        return;
    }
    let 弦 = 终点 - 起点;
    let 偏差 = |控制: Point| {
        if 弦.hypot() < 1e-12 {
            (控制 - 起点).hypot()
        } else {
            (控制 - 起点).cross(弦).abs() / 弦.hypot()
        }
    };
    let 偏差1 = 偏差(控制1);
    let 偏差2 = 偏差(控制2);
    if 偏差1.max(偏差2) <= 容差 {
        输出.push(终点);
        return;
    }
    let 曲线 = CubicBez::new(起点, 控制1, 控制2, 终点);
    let (左, 右) = 曲线.subdivide();
    展平三次(左.p0, 左.p1, 左.p2, 左.p3, 容差, 深度 + 1, 输出);
    展平三次(右.p0, 右.p1, 右.p2, 右.p3, 容差, 深度 + 1, 输出);
}

/// 轮廓转 flo_curves 路径（三次曲线；直线保留三等分控制点）。
fn 轮廓转flo(轮廓: &子路径) -> SimpleBezierPath {
    let 起点 = Coord2(轮廓.起点.x, 轮廓.起点.y);
    let mut 当前 = 起点;
    let mut 曲线 = Vec::with_capacity(轮廓.段.len());
    for 段 in &轮廓.段 {
        let 终点 = 段.终点();
        let 终点坐标 = Coord2(终点.x, 终点.y);
        let (控制1, 控制2) = match *段 {
            线段::直线(_) => {
                let 向量 = 终点坐标 - 当前;
                (当前 + 向量 * (1.0 / 3.0), 当前 + 向量 * (2.0 / 3.0))
            }
            线段::二次(控制, _) => {
                let 控制坐标 = Coord2(控制.x, 控制.y);
                (
                    当前 + (控制坐标 - 当前) * (2.0 / 3.0),
                    终点坐标 + (控制坐标 - 终点坐标) * (2.0 / 3.0),
                )
            }
            线段::三次(控制1, 控制2, _) => {
                (Coord2(控制1.x, 控制1.y), Coord2(控制2.x, 控制2.y))
            }
        };
        曲线.push((控制1, 控制2, 终点坐标));
        当前 = 终点坐标;
    }
    (起点, 曲线)
}

/// flo_curves 路径转轮廓，识别直线与二次曲线以保留紧凑表示。
fn flo转轮廓(路径: &SimpleBezierPath) -> 子路径 {
    let 起点 = 点自坐标(路径.0);
    let mut 当前 = 起点;
    let mut 段 = Vec::with_capacity(路径.1.len());
    for (控制1, 控制2, 终点) in &路径.1 {
        let (控制1, 控制2, 终点) = (点自坐标(*控制1), 点自坐标(*控制2), 点自坐标(*终点));
        段.push(识别曲线(当前, 控制1, 控制2, 终点));
        当前 = 终点;
    }
    子路径 {
        起点,
        段,
        闭合: true,
    }
}

/// 判断 flo 三次曲线实际是直线、二次还是三次。
fn 识别曲线(起点: Point, 控制1: Point, 控制2: Point, 终点: Point) -> 线段 {
    const 容差: f64 = 1e-6;
    let 三分之一 = 起点 + (终点 - 起点) * (1.0 / 3.0);
    let 三分之二 = 起点 + (终点 - 起点) * (2.0 / 3.0);
    if (控制1 - 三分之一).hypot() < 容差 && (控制2 - 三分之二).hypot() < 容差 {
        return 线段::直线(终点);
    }
    let 二次控制 = 起点 + (控制1 - 起点) * 1.5;
    let 二次控制2 = 终点 + (控制2 - 终点) * 1.5;
    if (二次控制 - 二次控制2).hypot() < 容差 {
        return 线段::二次(二次控制, 终点);
    }
    线段::三次(控制1, 控制2, 终点)
}

fn 点自坐标(坐标: Coord2) -> Point {
    Point::new(坐标.x(), 坐标.y())
}

/// kurbo 路径转子路径列表，按 MoveTo 分组。
fn bezipath转轮廓(路径: &BezPath) -> Vec<子路径> {
    let mut 结果: Vec<子路径> = Vec::new();
    let mut 当前: Option<子路径> = None;
    for 元素 in 路径.elements() {
        match *元素 {
            PathEl::MoveTo(起点) => {
                if let Some(轮廓) = 当前.take() {
                    结果.push(轮廓);
                }
                当前 = Some(子路径 {
                    起点,
                    段: Vec::new(),
                    闭合: false,
                });
            }
            PathEl::LineTo(终点) => {
                if let Some(轮廓) = 当前.as_mut() {
                    轮廓.段.push(线段::直线(终点));
                }
            }
            PathEl::QuadTo(控制, 终点) => {
                if let Some(轮廓) = 当前.as_mut() {
                    轮廓.段.push(线段::二次(控制, 终点));
                }
            }
            PathEl::CurveTo(控制1, 控制2, 终点) => {
                if let Some(轮廓) = 当前.as_mut() {
                    轮廓.段.push(线段::三次(控制1, 控制2, 终点));
                }
            }
            PathEl::ClosePath => {
                if let Some(轮廓) = 当前.as_mut() {
                    轮廓.闭合 = true;
                }
            }
        }
    }
    if let Some(轮廓) = 当前.take() {
        结果.push(轮廓);
    }
    结果
}

/// 按 y 分带的边索引，加速非零环绕采样。
struct 绕数索引 {
    下界: f64,
    带宽: f64,
    带: Vec<Vec<(Point, Point)>>,
    长边: Vec<(Point, Point)>,
}

impl 绕数索引 {
    /// 由展平多边形构建索引；退化输入返回空索引。
    fn 新建(多边形列表: &[Vec<Point>]) -> Self {
        let mut 最小y = f64::INFINITY;
        let mut 最大y = f64::NEG_INFINITY;
        for 多边形 in 多边形列表 {
            for 点 in 多边形 {
                最小y = 最小y.min(点.y);
                最大y = 最大y.max(点.y);
            }
        }
        const 带数: usize = 64;
        if !最小y.is_finite() || !最大y.is_finite() || 最大y <= 最小y {
            return Self {
                下界: 0.0,
                带宽: 1.0,
                带: Vec::new(),
                长边: Vec::new(),
            };
        }
        let 带宽 = (最大y - 最小y) / 带数 as f64;
        let mut 带: Vec<Vec<(Point, Point)>> = (0..带数).map(|_| Vec::new()).collect();
        let mut 长边 = Vec::new();
        for 多边形 in 多边形列表 {
            let 数量 = 多边形.len();
            for 序号 in 0..数量 {
                let 起点 = 多边形[序号];
                let 终点 = 多边形[(序号 + 1) % 数量];
                let (低, 高) = if 起点.y <= 终点.y {
                    (起点.y, 终点.y)
                } else {
                    (终点.y, 起点.y)
                };
                let 首 = 带编号(低, 最小y, 带宽, 带数);
                let 末 = 带编号(高, 最小y, 带宽, 带数);
                if 末 - 首 > 8 {
                    长边.push((起点, 终点));
                } else {
                    for 桶 in &mut 带[首..=末] {
                        桶.push((起点, 终点));
                    }
                }
            }
        }
        Self {
            下界: 最小y,
            带宽,
            带,
            长边,
        }
    }

    /// 非零环绕计数；与逐边扫描结果一致。
    fn 绕数(&self, 点: Point) -> i64 {
        let mut 结果 = 0i64;
        let mut 累计 = |起点: Point, 终点: Point| {
            if 起点.y <= 点.y {
                if 终点.y > 点.y && (终点 - 起点).cross(点 - 起点) > 0.0 {
                    结果 += 1;
                }
            } else if 终点.y <= 点.y && (终点 - 起点).cross(点 - 起点) < 0.0 {
                结果 -= 1;
            }
        };
        for &(起点, 终点) in &self.长边 {
            累计(起点, 终点);
        }
        if !self.带.is_empty() {
            let 桶 = 带编号(点.y, self.下界, self.带宽, self.带.len());
            for &(起点, 终点) in &self.带[桶] {
                累计(起点, 终点);
            }
        }
        结果
    }
}

/// 把 y 值映射到分带编号。
fn 带编号(值: f64, 下界: f64, 带宽: f64, 带数: usize) -> usize {
    (((值 - 下界) / 带宽).floor() as i64).clamp(0, 带数 as i64 - 1) as usize
}

/// 奇偶规则包含测试（展平多边形）。
fn 多边形包含(多边形: &[Point], 点: Point) -> bool {
    let mut 在内 = false;
    let 数量 = 多边形.len();
    for 序号 in 0..数量 {
        let 起点 = 多边形[序号];
        let 终点 = 多边形[(序号 + 1) % 数量];
        if (起点.y > 点.y) != (终点.y > 点.y) {
            let 交点x = 起点.x + (点.y - 起点.y) / (终点.y - 起点.y) * (终点.x - 起点.x);
            if 点.x < 交点x {
                在内 = !在内;
            }
        }
    }
    在内
}

/// 非零环绕简化：解析自交与重叠，返回互不重叠的规范朝向轮廓。
///
/// 与 skia-pathops `simplify()` 语义一致：以原始轮廓的全局绕数判定
/// 填充区域，再按嵌套深度规范朝向（外轮廓顺时针、内孔逆时针）。
fn 简化轮廓(轮廓: &[子路径], 精度: f64) -> Vec<子路径> {
    if 轮廓.is_empty() {
        return Vec::new();
    }
    let 展平容差 = (精度 * 0.2).max(0.005);
    let 多边形列表: Vec<Vec<Point>> = 轮廓.iter().map(|项| 项.展平(展平容差)).collect();
    let flo列表: Vec<SimpleBezierPath> = 轮廓.iter().map(轮廓转flo).collect();
    let mut 图 = GraphPath::from_merged_paths(flo列表.iter().map(|路径| (路径, PathLabel(0))));
    图.self_collide(精度);
    图.round(精度);
    let 边列表: Vec<_> = 图.all_edge_refs().collect();
    let 偏移 = (精度 * 5.0).max(0.02);
    let 索引 = 绕数索引::新建(&多边形列表);
    for 边引用 in 边列表 {
        let 边 = 图.get_edge(边引用);
        let 位置 = 边.point_at_pos(0.5);
        let 切线 = 边.tangent_at_pos(0.5);
        let 长度 = 切线.x().hypot(切线.y());
        if 长度 <= f64::EPSILON {
            图.set_edge_kind(边引用, GraphPathEdgeKind::Interior);
            continue;
        }
        let 法线 = Coord2(-切线.y() / 长度, 切线.x() / 长度);
        let 左 = 索引.绕数(点自坐标(位置 + 法线 * 偏移)) != 0;
        let 右 = 索引.绕数(点自坐标(位置 - 法线 * 偏移)) != 0;
        图.set_edge_kind(
            边引用,
            if 左 != 右 {
                GraphPathEdgeKind::Exterior
            } else {
                GraphPathEdgeKind::Interior
            },
        );
    }
    图.heal_exterior_gaps();
    let 输出: Vec<SimpleBezierPath> = 图.exterior_paths();
    规范化朝向(&输出.iter().map(flo转轮廓).collect::<Vec<_>>())
}

/// 把奇偶规则输出的轮廓规范为非零环绕朝向：
/// 外轮廓顺时针（有向面积为负）、内孔逆时针。
fn 规范化朝向(轮廓: &[子路径]) -> Vec<子路径> {
    let 有效: Vec<&子路径> = 轮廓
        .iter()
        .filter(|项| !项.段.is_empty() && 项.有向面积().abs() > 1e-6)
        .collect();
    if 有效.is_empty() {
        return Vec::new();
    }
    let 多边形列表: Vec<Vec<Point>> = 有效.iter().map(|项| 项.展平(0.01)).collect();
    let 朝向: Vec<bool> = 有效.iter().map(|项| 项.有向面积() > 0.0).collect();
    let mut 结果 = Vec::with_capacity(有效.len());
    for (序号, 项) in 有效.iter().enumerate() {
        let 采样点 = 项.内部采样点();
        let 深度 = 多边形列表
            .iter()
            .enumerate()
            .filter(|(其他, 多边形)| *其他 != 序号 && 多边形包含(多边形, 采样点))
            .count();
        let 应为正 = 深度 % 2 == 1;
        if 朝向[序号] == 应为正 {
            结果.push((**项).clone());
        } else {
            结果.push(项.反转());
        }
    }
    结果
}

/// 轮廓整体转二次（保留既有二次段）；三次段交给 kurbo 的 cu2qu 移植。
fn 轮廓转二次(轮廓: &子路径, 容差: f64) -> 子路径 {
    let mut 段 = Vec::new();
    for 段项 in &轮廓.段 {
        match *段项 {
            线段::直线(终点) => 段.push(线段::直线(终点)),
            线段::二次(控制, 终点) => 段.push(线段::二次(控制, 终点)),
            线段::三次(控制1, 控制2, 终点) => {
                let 起点 = 段.last().map_or(轮廓.起点, 线段::终点);
                for (_, _, 二次) in CubicBez::new(起点, 控制1, 控制2, 终点).to_quads(容差)
                {
                    段.push(线段::二次(二次.p1, 二次.p2));
                }
            }
        }
    }
    子路径 {
        起点: 轮廓.起点,
        段,
        闭合: 轮廓.闭合,
    }
}

/// 字体空间变换：x × 64，y = 896 - 64 × y。
fn 字体变换() -> Affine {
    Affine::new([每像素单位, 0.0, 0.0, -每像素单位, 0.0, 基线])
}

/// 转为字体空间：变换、三次转二次、反转方向（与 Cu2QuPen 的
/// `reverse_direction=True` 一致）。
fn 字体空间轮廓(轮廓: &子路径) -> 子路径 {
    let 变换后 = 轮廓.变换(字体变换());
    let 二次 = 轮廓转二次(&变换后, 1.0);
    二次.反转()
}

/// 字重内缩：外轮廓收缩、内孔扩张，保留原中心线。
fn 应用字重内缩(轮廓: &[子路径], 内缩: f64) -> 结果<Vec<子路径>> {
    let 已填充 = 简化轮廓(轮廓, 0.05);
    let 宽度 = 内缩 * 128.0;
    let 样式 = Stroke::new(宽度);
    let mut 描边: Vec<子路径> = Vec::new();
    for 项 in &已填充 {
        let 结果 = kurbo::stroke(项.转bezipath(), &样式, &StrokeOpts::default(), 0.05);
        描边.extend(bezipath转轮廓(&结果));
    }
    let 边界 = 简化轮廓(&描边, 0.05);
    let 已填充flo: Vec<SimpleBezierPath> = 已填充.iter().map(轮廓转flo).collect();
    let 边界flo: Vec<SimpleBezierPath> = 边界.iter().map(轮廓转flo).collect();
    let 调整后: Vec<子路径> = path_sub(&已填充flo, &边界flo, 0.05)
        .iter()
        .map(flo转轮廓)
        .collect();
    let 调整后 = 规范化朝向(&调整后);
    let 填充面积: f64 = 已填充.iter().map(子路径::有向面积).sum();
    let 调整面积: f64 = 调整后.iter().map(子路径::有向面积).sum();
    if 调整后.is_empty() || 调整面积.abs() < 填充面积.abs() * 0.55 {
        return Err(工具错误::新("字重调整损失过多可见轮廓"));
    }
    Ok(调整后.iter().map(|项| 轮廓转二次(项, 1.0)).collect())
}

/// 格式化坐标：值为 1/64 的整数倍，最多 6 位小数并去掉尾零。
fn 格式化数值(值: f64) -> String {
    if 值 == 0.0 {
        return "0".to_string();
    }
    if 值 == 值.trunc() && 值.abs() < 1e15 {
        return format!("{}", 值 as i64);
    }
    let mut 文本 = format!("{值:.6}");
    while 文本.ends_with('0') {
        文本.pop();
    }
    if 文本.ends_with('.') {
        文本.pop();
    }
    文本
}

/// XML 元素节点；属性保留原始限定名，命名空间仅用于判断是否 SVG。
struct 节点 {
    本地名: String,
    是svg: bool,
    外部命名空间: bool,
    属性: BTreeMap<String, String>,
    子: Vec<节点>,
}

/// 解析 XML 为元素树；仅保留元素与属性，忽略文本与注释。
fn 解析xml(文本: &str, 文件名: &str) -> 结果<节点> {
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
fn 解析路径数据(数据: &str, 文件名: &str) -> 结果<Vec<子路径>> {
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
fn 取值(abs: bool, 当前: Point, x: f64, y: f64) -> Point {
    if abs {
        Point::new(x, y)
    } else {
        Point::new(当前.x + x, 当前.y + y)
    }
}

/// 追加线段；缺少 MoveTo 时报错，`Z` 之后的绘制命令会从起点另起子路径。
fn 推段(
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
fn 段坐标有效(段: &线段) -> bool {
    let 有效 = |点: &Point| {
        点.x.is_finite() && 点.y.is_finite() && 点.x.abs() <= 坐标上限 && 点.y.abs() <= 坐标上限
    };
    match *段 {
        线段::直线(终点) => 有效(&终点),
        线段::二次(控制, 终点) => 有效(&控制) && 有效(&终点),
        线段::三次(控制1, 控制2, 终点) => 有效(&控制1) && 有效(&控制2) && 有效(&终点),
    }
}

/// 解析元素变换；只接受 matrix/translate/scale，其余明确报错。
fn 解析变换(值: &str, 文件名: &str) -> 结果<Affine> {
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
fn 解析style(值: Option<&String>) -> BTreeMap<String, String> {
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
fn 合并属性(节点: &节点) -> BTreeMap<String, String> {
    let mut 结果 = 节点.属性.clone();
    for (键, 值) in 解析style(节点.属性.get("style")) {
        结果.insert(键, 值);
    }
    结果
}

/// 读取数值属性，缺省时使用默认值；拒绝非有限或超幅值。
fn 数值属性(
    属性: &BTreeMap<String, String>, 键: &str, 默认: f64, 文件名: &str
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
fn 解析数值列表(文本: &str, 文件名: &str, 说明: &str) -> 结果<Vec<f64>> {
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
fn 元素子路径(节点: &节点, 文件名: &str) -> 结果<Vec<子路径>> {
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
fn 解析点列(文本: &str, 文件名: &str) -> 结果<Vec<Point>> {
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
fn 矩形路径(x0: f64, y0: f64, x1: f64, y1: f64, rx: f64, ry: f64) -> Vec<子路径> {
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
fn 椭圆路径(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<子路径> {
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
fn 视口路径() -> 子路径 {
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
fn 处理轮廓(
    轮廓: 子路径, 特效: bool, 文件名: &str, 收集: &mut Vec<子路径>
) -> 结果<()> {
    let Some((左, 上, 右, 下)) = 轮廓.包围盒() else {
        return Ok(());
    };
    if 右 <= 0.0 || 下 <= 0.0 || 左 >= 16.0 || 上 >= 16.0 {
        return Ok(());
    }
    if 特效 {
        return Err(工具错误::新(format!(
            "{文件名} 存在可见遮罩或裁切，不能直接转换为轮廓字体"
        )));
    }
    if 左.min(上) < -0.0625 || 右.max(下) > 16.0625 {
        return Err(工具错误::新(format!("{文件名} 的可见轮廓超出画布")));
    }
    收集.push(字体空间轮廓(&轮廓));
    Ok(())
}

/// 访问 SVG 元素树，按 Python 版规则收集可见轮廓。
fn 访问元素(
    节点: &节点,
    变换: Affine,
    特效: bool,
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
    let 自身 = 解析变换(
        节点.属性.get("transform").map_or("", String::as_str),
        文件名,
    )?;
    let 变换 = 变换 * 自身;
    let 特效 = 特效
        || ["mask", "clip-path", "filter"]
            .iter()
            .any(|键| 节点.属性.contains_key(*键));
    if 节点.本地名 == "svg" || 节点.本地名 == "g" {
        for 子 in &节点.子 {
            访问元素(子, 变换, 特效, 文件名, 收集)?;
        }
        return Ok(());
    }
    let 子路径列表 = 元素子路径(节点, 文件名)?;
    let 属性 = 合并属性(节点);
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
    for 子 in 子路径列表 {
        let mut 子 = 子.变换(变换);
        子.隐式闭合();
        处理轮廓(子, 特效, 文件名, 收集)?;
    }
    Ok(())
}

/// 读取 SVG 并校验画布，收集可见轮廓（SVG 用户坐标）。
fn 绘制svg(文本: &str, 文件名: &str, 收集: &mut Vec<子路径>) -> 结果<()> {
    let 根 = 解析xml(文本, 文件名)?;
    let 画布文本 = 根.属性.get("viewBox").map_or("0 0 16 16", String::as_str);
    let 画布 = 解析数值列表(画布文本, 文件名, " viewBox")?;
    if 画布.as_slice() != [0.0, 0.0, 16.0, 16.0] {
        return Err(工具错误::新(format!("{文件名} 需要 16 × 16 画布")));
    }
    访问元素(&根, Affine::IDENTITY, false, 文件名, 收集)
}

/// 递归收集 SVG 命名空间下所有 path 元素（自有字形按整棵树取用）。
fn 收集path元素(节点: &节点, 文件名: &str, 收集: &mut Vec<子路径>) -> 结果<()> {
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

/// 字体名称表内容。
struct 字体描述<'a> {
    族名: &'a str,
    样式名: &'a str,
    唯一标识: &'a str,
    全名: &'a str,
    ps名: &'a str,
    版本: &'a str,
    版权: &'a str,
    许可: &'a str,
    许可地址: &'a str,
}

/// 写入 TTF：glyf/loca、度量、cmap、名称与其余必需表。
fn 生成字体字节(
    名称表: &[String],
    轮廓表: &[Vec<子路径>],
    码点表: &[(u16, usize)],
    描述: &字体描述<'_>,
) -> 结果<Vec<u8>> {
    if 名称表.len() != 轮廓表.len() || 名称表.first().map(String::as_str) != Some(".notdef")
    {
        return Err(工具错误::新("字形表与名称表不一致"));
    }
    let mut 构建器 = GlyfLocaBuilder::new();
    let mut 包围盒列表: Vec<Option<(i16, i16, i16, i16)>> = Vec::with_capacity(轮廓表.len());
    let mut 最大点数 = 0u16;
    let mut 最大轮廓数 = 0u16;
    for 轮廓 in 轮廓表 {
        if 轮廓.is_empty() {
            构建器
                .add_glyph(&Glyph::Empty)
                .map_err(|错误| 工具错误::新(format!("字形构建失败：{错误}")))?;
            包围盒列表.push(None);
            continue;
        }
        let 路径 = 合并路径(轮廓);
        let 字形 = SimpleGlyph::from_bezpath(&路径)
            .map_err(|错误| 工具错误::新(format!("字形编译失败：{错误:?}")))?;
        if 字形.contours.len() >= i16::MAX as usize {
            return Err(工具错误::新(format!(
                "字形轮廓数过多：{}",
                字形.contours.len()
            )));
        }
        let 点数: usize = 字形.contours.iter().map(|项| 项.len()).sum();
        最大点数 = 最大点数.max(u16::try_from(点数).unwrap_or(u16::MAX));
        最大轮廓数 = 最大轮廓数.max(u16::try_from(字形.contours.len()).unwrap_or(u16::MAX));
        let 盒 = 字形.bbox;
        包围盒列表.push(Some((盒.x_min, 盒.y_min, 盒.x_max, 盒.y_max)));
        构建器
            .add_glyph(&Glyph::Simple(字形))
            .map_err(|错误| 工具错误::新(format!("字形构建失败：{错误}")))?;
    }
    let (glyf, loca, 定位格式) = 构建器.build();

    let 左边界: Vec<i16> = 包围盒列表
        .iter()
        .map(|盒| 盒.map_or(0, |值| 值.0))
        .collect();
    let 前进列表: Vec<u16> = vec![前进宽度; 名称表.len()];
    let 度量数 = 压缩度量数(&前进列表).max(1);
    let 长度量: Vec<LongMetric> = (0..度量数)
        .map(|序号| LongMetric::new(前进宽度, 左边界[序号]))
        .collect();
    let 余留边界: Vec<i16> = 左边界[度量数..].to_vec();
    let hmtx = Hmtx::new(长度量, 余留边界);

    let 非空: Vec<usize> = (0..名称表.len())
        .filter(|序号| 包围盒列表[*序号].is_some())
        .collect();
    let 最小左 = 非空.iter().map(|序号| 左边界[*序号]).min().unwrap_or(0);
    let 最小右 = 非空
        .iter()
        .map(|序号| {
            let 盒 = 包围盒列表[*序号].expect("非空字形有包围盒");
            前进宽度 as i32 - 左边界[*序号] as i32 - (盒.2 as i32 - 盒.0 as i32)
        })
        .min()
        .map_or(0, |值| 值 as i16);
    let 最大延伸 = 非空
        .iter()
        .map(|序号| {
            let 盒 = 包围盒列表[*序号].expect("非空字形有包围盒");
            左边界[*序号] as i32 + (盒.2 as i32 - 盒.0 as i32)
        })
        .max()
        .map_or(0, |值| 值 as i16);
    let hhea = Hhea {
        ascender: 896.into(),
        descender: (-128).into(),
        line_gap: 0.into(),
        advance_width_max: 前进宽度.into(),
        min_left_side_bearing: 最小左.into(),
        min_right_side_bearing: 最小右.into(),
        x_max_extent: 最大延伸.into(),
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        number_of_h_metrics: u16::try_from(度量数).unwrap_or(u16::MAX),
    };

    let maxp = Maxp {
        num_glyphs: u16::try_from(名称表.len()).unwrap_or(u16::MAX),
        max_points: Some(最大点数),
        max_contours: Some(最大轮廓数),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(2),
        max_twilight_points: Some(0),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(0),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };

    let mut 全局盒 = (0i32, 0i32, 0i32, 0i32);
    let mut 有盒 = false;
    for 盒 in 包围盒列表.iter().flatten() {
        if !有盒 {
            全局盒 = (盒.0 as i32, 盒.1 as i32, 盒.2 as i32, 盒.3 as i32);
            有盒 = true;
        } else {
            全局盒.0 = 全局盒.0.min(盒.0 as i32);
            全局盒.1 = 全局盒.1.min(盒.1 as i32);
            全局盒.2 = 全局盒.2.max(盒.2 as i32);
            全局盒.3 = 全局盒.3.max(盒.3 as i32);
        }
    }
    let head = Head {
        font_revision: Fixed::from_f64(1.0),
        flags: Flags::BASELINE_AT_Y_0 | Flags::LSB_AT_X_0,
        units_per_em: 1024,
        created: LongDateTime::new(固定时间戳),
        modified: LongDateTime::new(固定时间戳),
        x_min: 全局盒.0 as i16,
        y_min: 全局盒.1 as i16,
        x_max: 全局盒.2 as i16,
        y_max: 全局盒.3 as i16,
        mac_style: MacStyle::empty(),
        lowest_rec_ppem: 3,
        font_direction_hint: 2,
        index_to_loc_format: 定位格式 as i16,
        ..Default::default()
    };

    let cmap子表 = 构建cmap4(码点表);
    let cmap = Cmap::new(vec![
        EncodingRecord::new(PlatformId::Unicode, 3, cmap子表.clone()),
        EncodingRecord::new(PlatformId::Windows, 1, cmap子表),
    ]);

    let 记录 = vec![
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::COPYRIGHT_NOTICE,
            描述.版权.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::FAMILY_NAME,
            描述.族名.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::SUBFAMILY_NAME,
            描述.样式名.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::UNIQUE_ID,
            描述.唯一标识.to_string().into(),
        ),
        NameRecord::new(3, 1, 0x409, NameId::FULL_NAME, 描述.全名.to_string().into()),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::VERSION_STRING,
            描述.版本.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::POSTSCRIPT_NAME,
            描述.ps名.to_string().into(),
        ),
        NameRecord::new(3, 1, 0x409, NameId::new(13), 描述.许可.to_string().into()),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::new(14),
            描述.许可地址.to_string().into(),
        ),
    ];
    let name = Name::new(记录);

    let 最小码点 = 码点表.iter().map(|项| 项.0).min().unwrap_or(0);
    let 最大码点 = 码点表.iter().map(|项| 项.0).max().unwrap_or(0);
    let os2 = Os2 {
        us_weight_class: 400,
        us_width_class: 5,
        fs_type: 4,
        ach_vend_id: Tag::new(b"????"),
        fs_selection: SelectionFlags::empty(),
        us_first_char_index: 最小码点,
        us_last_char_index: 最大码点,
        s_typo_ascender: 896,
        s_typo_descender: -128,
        s_typo_line_gap: 0,
        us_win_ascent: 896,
        us_win_descent: 128,
        ul_code_page_range_1: Some(0),
        ul_code_page_range_2: Some(0),
        us_default_char: Some(0),
        us_break_char: Some(32),
        us_max_context: Some(0),
        sx_height: Some(0),
        s_cap_height: Some(0),
        x_avg_char_width: 前进宽度 as i16,
        ul_unicode_range_2: 1 << 28,
        ..Default::default()
    };

    let post = Post::new_v2(名称表.iter().map(String::as_str));

    let mut 字体构建 = FontBuilder::new();
    字体构建
        .add_table(&head)
        .and_then(|项| 项.add_table(&hhea))
        .and_then(|项| 项.add_table(&hmtx))
        .and_then(|项| 项.add_table(&maxp))
        .and_then(|项| 项.add_table(&cmap))
        .and_then(|项| 项.add_table(&name))
        .and_then(|项| 项.add_table(&os2))
        .and_then(|项| 项.add_table(&post))
        .and_then(|项| 项.add_table(&glyf))
        .and_then(|项| 项.add_table(&loca))
        .map_err(|错误| 工具错误::新(format!("字体表写入失败：{错误}")))?;
    let mut 字节 = 字体构建.build();
    修补os2版本(&mut 字节, 3)?;
    Ok(字节)
}

/// 把 OS/2 版本改写为指定值并同步校验和（write-fonts 只能按字段算出 4）。
fn 修补os2版本(字节: &mut [u8], 版本: u16) -> 结果<()> {
    let 表数 = u16::from_be_bytes(
        字节
            .get(4..6)
            .ok_or_else(|| 工具错误::新("字体缺少表目录"))?
            .try_into()
            .expect("表数"),
    ) as usize;
    let mut os2记录 = None;
    let mut head偏移 = None;
    for 序号 in 0..表数 {
        let 记录 = 12 + 序号 * 16;
        let 标签 = 字节
            .get(记录..记录 + 4)
            .ok_or_else(|| 工具错误::新("字体表目录不完整"))?;
        let 偏移 = u32::from_be_bytes(
            字节
                .get(记录 + 8..记录 + 12)
                .ok_or_else(|| 工具错误::新("字体表目录不完整"))?
                .try_into()
                .expect("表偏移"),
        ) as usize;
        let 长度 = u32::from_be_bytes(
            字节
                .get(记录 + 12..记录 + 16)
                .ok_or_else(|| 工具错误::新("字体表目录不完整"))?
                .try_into()
                .expect("表长度"),
        ) as usize;
        if 标签 == b"OS/2" {
            os2记录 = Some((记录, 偏移, 长度));
        } else if 标签 == b"head" {
            head偏移 = Some(偏移);
        }
    }
    let (记录, 偏移, 长度) = os2记录.ok_or_else(|| 工具错误::新("字体缺少 OS/2 表"))?;
    if 长度 < 2 || 偏移.checked_add(长度).is_none_or(|结束| 结束 > 字节.len()) {
        return Err(工具错误::新("OS/2 表超出字体范围"));
    }
    字节[偏移..偏移 + 2].copy_from_slice(&版本.to_be_bytes());
    let 表校验 = 表校验和(
        字节
            .get(偏移..偏移 + 长度)
            .ok_or_else(|| 工具错误::新("OS/2 表超出字体范围"))?,
    );
    字节[记录 + 4..记录 + 8].copy_from_slice(&表校验.to_be_bytes());
    let head偏移 = head偏移.ok_or_else(|| 工具错误::新("字体缺少 head 表"))?;
    let 调整位置 = head偏移 + 8;
    字节
        .get_mut(调整位置..调整位置 + 4)
        .ok_or_else(|| 工具错误::新("head 表超出字体范围"))?
        .fill(0);
    let 总校验 = 表校验和(字节);
    let 调整 = 0xB1B0_AFBAu32.wrapping_sub(总校验);
    字节[调整位置..调整位置 + 4].copy_from_slice(&调整.to_be_bytes());
    Ok(())
}

/// SFNT 校验和：按 4 字节大端累加，尾部补零。
fn 表校验和(字节: &[u8]) -> u32 {
    let mut 和 = 0u32;
    let mut 序号 = 0;
    while 序号 < 字节.len() {
        let mut 词 = [0u8; 4];
        let 剩余 = (字节.len() - 序号).min(4);
        词[..剩余].copy_from_slice(&字节[序号..序号 + 剩余]);
        和 = 和.wrapping_add(u32::from_be_bytes(词));
        序号 += 4;
    }
    和
}

/// 计算可省略尾部等宽度量的 numberOfHMetrics。
fn 压缩度量数(前进列表: &[u16]) -> usize {
    let 末前进 = 前进列表.last().copied().unwrap_or(0);
    let mut 度量数 = 前进列表.len();
    while 度量数 > 1 && 前进列表[度量数 - 2] == 末前进 {
        度量数 -= 1;
    }
    度量数
}

/// 合并轮廓为单个 BezPath。
fn 合并路径(轮廓: &[子路径]) -> BezPath {
    let mut 路径 = BezPath::new();
    for 项 in 轮廓 {
        路径.extend(项.转bezipath());
    }
    路径
}

/// 构建 cmap format 4 子表。
fn 构建cmap4(码点表: &[(u16, usize)]) -> CmapSubtable {
    let mut 排序: Vec<(u16, usize)> = 码点表.to_vec();
    排序.sort_unstable();
    let mut 结束码 = Vec::new();
    let mut 起始码 = Vec::new();
    let mut 差值 = Vec::new();
    let mut 范围偏移 = Vec::new();
    let mut 序号 = 0;
    while 序号 < 排序.len() {
        let 起点 = 排序[序号].0;
        let 首字形 = 排序[序号].1;
        let mut 终点 = 起点;
        let mut 下一 = 序号 + 1;
        while 下一 < 排序.len()
            && 排序[下一].0 == 终点.wrapping_add(1)
            && 排序[下一].1 == 首字形 + (终点 - 起点) as usize + 1
        {
            终点 = 排序[下一].0;
            下一 += 1;
        }
        起始码.push(起点);
        结束码.push(终点);
        差值.push((首字形 as i64 - 起点 as i64) as i16);
        范围偏移.push(0);
        序号 = 下一;
    }
    起始码.push(0xFFFF);
    结束码.push(0xFFFF);
    差值.push(1);
    范围偏移.push(0);
    CmapSubtable::format_4(0, 结束码, 起始码, 差值, 范围偏移, Vec::new())
}

/// `adwcode 生成自有字形` 的入口。
pub fn 自有入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    if let Some(其他) = 参数.first() {
        return Err(工具错误::新(format!("未知参数：{其他}")));
    }
    let 目录 = 根目录.join("产品图标").join("符号图标");
    let mut 源文件: Vec<PathBuf> = std::fs::read_dir(&目录)
        .map_err(|错误| 工具错误::带来源(format!("无法读取 {}", 目录.display()), 错误))?
        .filter_map(|项| 项.ok())
        .map(|项| 项.path())
        .filter(|路径| 路径.extension().is_some_and(|扩展| 扩展 == "svg"))
        .collect();
    源文件.sort_by_key(|路径| 路径.file_name().map(|名| 名.to_os_string()));
    let mut 名称表 = vec![".notdef".to_string()];
    let mut 轮廓表: Vec<Vec<子路径>> = vec![Vec::new()];
    let mut 码点表: Vec<(u16, usize)> = Vec::new();
    for 文件 in &源文件 {
        let 文件名 = 文件
            .file_name()
            .map_or_else(String::new, |名| 名.to_string_lossy().to_string());
        let 文本 = std::fs::read_to_string(文件).map_err(|错误| {
            工具错误::带来源(format!("无法读取 {}", 文件.display()), 错误)
        })?;
        let 根 = 解析xml(&文本, &文件名)?;
        let mut 轮廓 = Vec::new();
        收集path元素(&根, &文件名, &mut 轮廓)?;
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
        许可地址: "https://github.com/KOTONEX/AdwCode/blob/main/产品图标/LICENSE",
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
    let mut 操作 = Vec::new();
    let mut 消息 = Vec::new();
    for 字体 in 字体列表 {
        let id = 字符串字段(字体, "id")?;
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
        let mut 名称表 = vec![".notdef".to_string()];
        let mut 轮廓表: Vec<Vec<子路径>> = vec![Vec::new()];
        let mut 码点表: Vec<(u16, usize)> = Vec::new();
        for 条目 in 字形条目 {
            let 文件 = 字符串字段(条目, "file")?;
            let 源路径 = 图标目录.join(校验相对路径(&文件, "glyphs[].file")?);
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
            绘制svg(&文本, &文件名, &mut 轮廓)?;
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

/// 导出陈列预览并清理过期文件。
fn 导出预览(
    图标目录: &Path,
    字体id: &Path,
    字形条目: &[Value],
    轮廓表: &[Vec<子路径>],
    许可: &str,
    署名: &str,
) -> 结果<Vec<(PathBuf, Option<Vec<u8>>)>> {
    let mut 操作 = Vec::new();
    let 预览目录 = 图标目录.join("渲染图标").join(字体id);
    std::fs::create_dir_all(&预览目录).map_err(|错误| {
        工具错误::带来源(format!("无法创建 {}", 预览目录.display()), 错误)
    })?;
    let 预览变换 = 字体变换().inverse();
    let mut 期望: Vec<String> = Vec::new();
    for (序号, 条目) in 字形条目.iter().enumerate() {
        let 码点 = 字符串字段(条目, "codepoint")?;
        let 文件名 = format!("{码点}.svg");
        let 轮廓 = &轮廓表[序号 + 1];
        let mut 命令 = String::new();
        for 项 in 轮廓 {
            命令.push_str(&项.变换(预览变换).svg命令());
        }
        let 内容 = format!(
            "<!-- SPDX-License-Identifier: {许可} -->\n<!-- 署名：{署名}；由 AdwCode 从字体轮廓生成，来源见 来源.json。 -->\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" width=\"16\" height=\"16\"><path fill=\"currentColor\" d=\"{命令}\"/></svg>\n"
        );
        操作.push((预览目录.join(&文件名), Some(内容.into_bytes())));
        期望.push(文件名);
    }
    let 目录项 = std::fs::read_dir(&预览目录).map_err(|错误| {
        工具错误::带来源(format!("无法读取 {}", 预览目录.display()), 错误)
    })?;
    for 项 in 目录项 {
        let 项 = 项?;
        let 路径 = 项.path();
        let 名字 = 路径
            .file_name()
            .map_or_else(String::new, |名| 名.to_string_lossy().to_string());
        if 路径.extension().is_some_and(|扩展| 扩展 == "svg") && !期望.contains(&名字) {
            操作.push((路径, None));
        }
    }
    Ok(操作)
}

/// 读取字符串字段。
fn 字符串字段(值: &Value, 键: &str) -> 结果<String> {
    值[键]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| 工具错误::新(format!("来源.json 缺少字段：{键}")))
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

#[cfg(test)]
mod 测试 {
    use super::*;
    use skrifa::FontRef;
    use skrifa::MetadataProvider;
    use skrifa::raw::TableProvider;
    use std::path::Path as 路径;

    /// 建立含图标资产的临时仓库根目录。
    fn 测试根() -> tempfile::TempDir {
        let 临时 = tempfile::tempdir().expect("创建临时目录");
        let 产品目录 = 临时.path().join("产品图标");
        std::fs::create_dir_all(&产品目录).expect("创建产品图标目录");
        let 源图标 = 路径::new(env!("CARGO_MANIFEST_DIR")).join("产品图标");
        std::fs::copy(源图标.join("来源.json"), 产品目录.join("来源.json")).expect("复制来源.json");
        复制目录(&源图标.join("符号图标"), &产品目录.join("符号图标"));
        复制目录(&源图标.join("导入图标"), &产品目录.join("导入图标"));
        临时
    }

    fn 复制目录(来源: &路径, 目标: &路径) {
        std::fs::create_dir_all(目标).expect("创建目录");
        for 项 in std::fs::read_dir(来源).expect("读取目录") {
            let 项 = 项.expect("目录项");
            let 目标路径 = 目标.join(项.file_name());
            if 项.path().is_dir() {
                复制目录(&项.path(), &目标路径);
            } else {
                std::fs::copy(项.path(), 目标路径).expect("复制文件");
            }
        }
    }

    fn 读取字体(路径: &路径) -> Vec<u8> {
        std::fs::read(路径).expect("读取字体")
    }

    #[test]
    fn 同起终点的闭环曲线不能被展平丢失() {
        let mut 点 = vec![Point::ZERO];
        展平三次(
            Point::ZERO,
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
            Point::ZERO,
            0.1,
            0,
            &mut 点,
        );
        assert!(点.len() > 4);
        assert!(点.iter().any(|点| 点.x > 1.0 && 点.y > 1.0));
    }

    #[test]
    fn 拒绝多个根未闭合xml和越界起点() {
        for 文本 in ["<svg/><svg/>", "<svg/ ><svg>"] {
            assert!(解析xml(文本, "测试").is_err());
        }
        assert!(解析路径数据("M1e100 0", "测试").is_err());
    }

    #[test]
    fn 损坏字体表返回错误而不越界访问() {
        let mut 字节 = vec![0; 44];
        字节[4..6].copy_from_slice(&2u16.to_be_bytes());
        字节[12..16].copy_from_slice(b"OS/2");
        字节[20..24].copy_from_slice(&100u32.to_be_bytes());
        字节[24..28].copy_from_slice(&2u32.to_be_bytes());
        字节[28..32].copy_from_slice(b"head");
        assert!(修补os2版本(&mut 字节, 3).is_err());
    }

    #[test]
    fn 自有字形数量与映射正确() {
        let 根 = 测试根();
        自有入口(根.path(), &[]).expect("生成自有字形");
        let 数据 = 读取字体(&根.path().join("产品图标/adwcode-符号.ttf"));
        let 字体 = FontRef::new(&数据).expect("解析自有字体");
        assert_eq!(字体.maxp().expect("maxp").num_glyphs(), 10);
        let 映射: Vec<(u32, skrifa::GlyphId)> = 字体.charmap().mappings().collect();
        assert_eq!(映射.len(), 9);
        assert!(映射.iter().any(|(码点, _)| *码点 == 0xF001));
        assert!(映射.iter().any(|(码点, _)| *码点 == 0xF009));
    }

    #[test]
    fn 轮廓转二次不会过度细分() {
        let 轮廓 = 子路径 {
            起点: Point::new(4.96875, 1.003906),
            段: vec![线段::三次(
                Point::new(3.324219, 1.003906),
                Point::new(1.96875, 2.359375),
                Point::new(1.96875, 4.003906),
            )],
            闭合: false,
        };
        let 二次 = 轮廓转二次(&轮廓, 1.0);
        assert!(
            二次.段.len() <= 4,
            "普通三次曲线不应过度细分：{}",
            二次.段.len()
        );
    }

    #[test]
    fn 导入字形分组与映射正确() {
        let 根 = 测试根();
        导入入口(根.path(), &[]).expect("生成导入字形");
        for (文件, 字形数, 映射数) in [
            ("adwaita-symbols.ttf", 38, 37),
            ("builder-symbols.ttf", 26, 25),
            ("morewaita-symbols.ttf", 4, 3),
        ] {
            let 数据 = 读取字体(&根.path().join("产品图标").join(文件));
            let 字体 = FontRef::new(&数据).expect("解析导入字体");
            assert_eq!(
                字体.maxp().expect("maxp").num_glyphs(),
                字形数,
                "{文件} 字形数"
            );
            let 映射 = 字体.charmap().mappings().count();
            assert_eq!(映射, 映射数, "{文件} 映射数");
            let os2 = 字体.os2().expect("OS/2");
            assert_eq!(os2.version(), 3, "{文件} 的 OS/2 版本");
        }
    }

    #[test]
    fn 生成结果确定() {
        let 根一 = 测试根();
        let 根二 = 测试根();
        自有入口(根一.path(), &[]).expect("生成自有字形一");
        自有入口(根二.path(), &[]).expect("生成自有字形二");
        assert_eq!(
            读取字体(&根一.path().join("产品图标/adwcode-符号.ttf")),
            读取字体(&根二.path().join("产品图标/adwcode-符号.ttf")),
            "自有字形字体字节不一致"
        );
        导入入口(根一.path(), &[]).expect("生成导入字形一");
        导入入口(根二.path(), &[]).expect("生成导入字形二");
        for 文件 in [
            "adwaita-symbols.ttf",
            "builder-symbols.ttf",
            "morewaita-symbols.ttf",
        ] {
            assert_eq!(
                读取字体(&根一.path().join("产品图标").join(文件)),
                读取字体(&根二.path().join("产品图标").join(文件)),
                "{文件} 字节不一致"
            );
        }
        assert_eq!(
            std::fs::read(
                根一
                    .path()
                    .join("产品图标/渲染图标/adwcode-adwaita/e300.svg")
            )
            .expect("读取预览一"),
            std::fs::read(
                根二
                    .path()
                    .join("产品图标/渲染图标/adwcode-adwaita/e300.svg")
            )
            .expect("读取预览二"),
            "预览字节不一致"
        );
    }

    #[test]
    fn 来源路径拒绝越界() {
        for 文本 in ["", "../x.svg", "/etc/passwd", "a/../../b.svg"] {
            assert!(校验相对路径(文本, "glyphs[].file").is_err(), "{文本}");
        }
        assert!(校验相对路径("导入图标/adwaita/x.svg", "glyphs[].file").is_ok());
    }

    #[test]
    fn 闭合后绘制命令另起子路径() {
        let 子路径列表 = 解析路径数据("M0 0L1 0Z L2 2", "测试.svg").expect("解析路径数据");
        assert_eq!(子路径列表.len(), 2);
        assert!(子路径列表[0].闭合);
        assert_eq!(子路径列表[0].段.len(), 1);
        assert_eq!(子路径列表[1].起点, Point::new(0.0, 0.0));
        assert_eq!(子路径列表[1].段.len(), 1);
        assert_eq!(子路径列表[1].段[0].终点(), Point::new(2.0, 2.0));
    }

    #[test]
    fn 路径数据拒绝超幅坐标() {
        assert!(解析路径数据("M0 0L1e100 0", "测试.svg").is_err());
        assert!(解析路径数据("M0 0LNaN 0", "测试.svg").is_err());
    }

    #[test]
    fn xml嵌套过深报错() {
        let 深 = 最大嵌套深度 + 1;
        let 文本 = format!(
            "<svg xmlns=\"{SVG命名空间}\">{}{}</svg>",
            "<g>".repeat(深),
            "</g>".repeat(深)
        );
        assert!(解析xml(&文本, "测试.svg").is_err());
    }

    #[test]
    fn 压缩度量数按末尾等宽收敛() {
        assert_eq!(压缩度量数(&[1024, 1024, 512, 512]), 3);
        assert_eq!(压缩度量数(&[1024, 1024, 1024]), 1);
        assert_eq!(压缩度量数(&[512, 1024]), 2);
    }

    fn 逐边绕数(多边形列表: &[Vec<Point>], 点: Point) -> i64 {
        let mut 结果 = 0i64;
        for 多边形 in 多边形列表 {
            let 数量 = 多边形.len();
            for 序号 in 0..数量 {
                let 起点 = 多边形[序号];
                let 终点 = 多边形[(序号 + 1) % 数量];
                if 起点.y <= 点.y {
                    if 终点.y > 点.y && (终点 - 起点).cross(点 - 起点) > 0.0 {
                        结果 += 1;
                    }
                } else if 终点.y <= 点.y && (终点 - 起点).cross(点 - 起点) < 0.0 {
                    结果 -= 1;
                }
            }
        }
        结果
    }

    #[test]
    fn 绕数索引与逐边扫描一致() {
        let 多边形列表 = vec![
            vec![
                Point::new(0.0, 0.0),
                Point::new(4.0, 0.0),
                Point::new(4.0, 4.0),
                Point::new(0.0, 4.0),
            ],
            vec![
                Point::new(1.0, 1.0),
                Point::new(1.0, 3.0),
                Point::new(3.0, 3.0),
                Point::new(3.0, 1.0),
            ],
            vec![
                Point::new(-10.0, -10.0),
                Point::new(-8.0, -10.0),
                Point::new(-9.0, -8.0),
            ],
        ];
        let 索引 = 绕数索引::新建(&多边形列表);
        for 点 in [
            Point::new(0.5, 0.5),
            Point::new(2.0, 2.0),
            Point::new(1.0, 2.0),
            Point::new(5.0, 2.0),
            Point::new(-9.0, -9.5),
            Point::new(2.0, -1.0),
        ] {
            assert_eq!(索引.绕数(点), 逐边绕数(&多边形列表, 点), "{点:?}");
        }
    }
}
