// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

use super::{基线, 展平最大深度, 每像素单位};
use crate::错误::{工具错误, 结果};
use flo_curves::Coord2;
use flo_curves::Coordinate2D;
use flo_curves::bezier::path::{
    GraphPath, GraphPathEdgeKind, PathLabel, SimpleBezierPath, path_sub,
};
use flo_curves::bezier::{BezierCurve, NormalCurve};
use kurbo::{Affine, BezPath, CubicBez, PathEl, Point, QuadBez, Stroke, StrokeOpts, Vec2};
use kurbo::{ParamCurve, ParamCurveDeriv, Shape};

/// 一段轮廓线段；坐标位于所在坐标系。
#[derive(Clone, Copy, Debug)]
pub(super) enum 线段 {
    直线(Point),
    二次(Point, Point),
    三次(Point, Point, Point),
}

impl 线段 {
    /// 线段终点。
    pub(super) fn 终点(&self) -> Point {
        match *self {
            线段::直线(终点) | 线段::二次(_, 终点) | 线段::三次(_, _, 终点) => {
                终点
            }
        }
    }
}

/// 一条轮廓：起点加有序线段，闭合标志表示 SVG 的 `Z`。
#[derive(Clone, Debug)]
pub(super) struct 子路径 {
    pub(super) 起点: Point,
    pub(super) 段: Vec<线段>,
    pub(super) 闭合: bool,
}

impl 子路径 {
    /// 轮廓最后一段的终点；空轮廓返回起点。
    pub(super) fn 终点(&self) -> Point {
        self.段.last().map_or(self.起点, 线段::终点)
    }

    /// 若未显式闭合，补一条回到起点的直线（字形填充按闭合处理）。
    pub(super) fn 隐式闭合(&mut self) {
        if !self.闭合 && self.终点() != self.起点 {
            self.段.push(线段::直线(self.起点));
        }
        self.闭合 = true;
    }

    /// 应用仿射变换。
    pub(super) fn 变换(&self, 变换: Affine) -> Self {
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
    pub(super) fn 反转(&self) -> Self {
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
    pub(super) fn 转bezipath(&self) -> BezPath {
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
    pub(super) fn 描边bezipath(&self) -> BezPath {
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
    pub(super) fn 包围盒(&self) -> Option<(f64, f64, f64, f64)> {
        if self.段.is_empty() {
            return None;
        }
        let 矩形 = self.转bezipath().bounding_box();
        Some((矩形.x0, 矩形.y0, 矩形.x1, 矩形.y1))
    }

    /// 有向面积；正为逆时针（y 向上）。
    pub(super) fn 有向面积(&self) -> f64 {
        self.转bezipath().area()
    }

    /// 自适应展平为多边形点列（起点开始，不含重复终点）。
    pub(super) fn 展平(&self, 容差: f64) -> Vec<Point> {
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
    pub(super) fn 内部采样点(&self) -> Point {
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
    pub(super) fn svg命令(&self) -> String {
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
pub(super) fn 展平二次(
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
pub(super) fn 展平三次(
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
pub(super) fn 轮廓转flo(轮廓: &子路径) -> SimpleBezierPath {
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
pub(super) fn flo转轮廓(路径: &SimpleBezierPath) -> 子路径 {
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
pub(super) fn 识别曲线(起点: Point, 控制1: Point, 控制2: Point, 终点: Point) -> 线段 {
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

pub(super) fn 点自坐标(坐标: Coord2) -> Point {
    Point::new(坐标.x(), 坐标.y())
}

/// kurbo 路径转子路径列表，按 MoveTo 分组。
pub(super) fn bezipath转轮廓(路径: &BezPath) -> Vec<子路径> {
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
pub(super) struct 绕数索引 {
    下界: f64,
    带宽: f64,
    带: Vec<Vec<(Point, Point)>>,
    长边: Vec<(Point, Point)>,
}

impl 绕数索引 {
    /// 由展平多边形构建索引；退化输入返回空索引。
    pub(super) fn 新建(多边形列表: &[Vec<Point>]) -> Self {
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
    pub(super) fn 绕数(&self, 点: Point) -> i64 {
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
pub(super) fn 带编号(值: f64, 下界: f64, 带宽: f64, 带数: usize) -> usize {
    (((值 - 下界) / 带宽).floor() as i64).clamp(0, 带数 as i64 - 1) as usize
}

/// 奇偶规则包含测试（展平多边形）。
pub(super) fn 多边形包含(多边形: &[Point], 点: Point) -> bool {
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
pub(super) fn 简化轮廓(轮廓: &[子路径], 精度: f64) -> Vec<子路径> {
    按填充规则简化(轮廓, 精度, false)
}

pub(super) fn 按填充规则简化(
    轮廓: &[子路径], 精度: f64, 奇偶: bool
) -> Vec<子路径> {
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
        let 填充 = |点| {
            let 绕数 = 索引.绕数(点);
            if 奇偶 { 绕数 % 2 != 0 } else { 绕数 != 0 }
        };
        let 左 = 填充(点自坐标(位置 + 法线 * 偏移));
        let 右 = 填充(点自坐标(位置 - 法线 * 偏移));
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
pub(super) fn 规范化朝向(轮廓: &[子路径]) -> Vec<子路径> {
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
pub(super) fn 轮廓转二次(轮廓: &子路径, 容差: f64) -> 子路径 {
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
pub(super) fn 字体变换() -> Affine {
    Affine::new([每像素单位, 0.0, 0.0, -每像素单位, 0.0, 基线])
}

/// 转为字体空间：变换、三次转二次、反转方向（与 Cu2QuPen 的
/// `reverse_direction=True` 一致）。
pub(super) fn 字体空间轮廓(轮廓: &子路径) -> 子路径 {
    let 变换后 = 轮廓.变换(字体变换());
    let 二次 = 轮廓转二次(&变换后, 1.0);
    二次.反转()
}

/// 字重内缩：外轮廓收缩、内孔扩张，保留原中心线。
pub(super) fn 应用字重内缩(轮廓: &[子路径], 内缩: f64) -> 结果<Vec<子路径>> {
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
pub(super) fn 格式化数值(值: f64) -> String {
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
