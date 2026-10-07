// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! AdwCode 的 Adwaita 颜色角色。
//!
//! 来源
//! ----
//! - libadwaita 1.10 CSS 变量（GNOME 51）：
//!   <https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.10/css-variables.html>
//!   取值已与系统 `libadwaita-1.so.0` 中提取的 `/org/gnome/Adwaita/styles/gtk.css`
//!   逐项核对。
//! - GtkSourceView 5 的 `Adwaita` / `Adwaita-dark` 样式方案（LGPL-2.1-or-later），
//!   存放于 `源码/GtkSourceView方案/`，用于编辑器默认值。
//!
//! 8 位十六进制颜色写作 `#rrggbbaa`；不透明颜色写作 `#rrggbb`。

use crate::有序映射::有序映射;
use crate::错误::{工具错误, 结果};

/// 当前启用的强调色名称。
pub const 强调色名称: [&str; 1] = ["blue"];

#[must_use]
pub fn 强调色标签(名称: &str) -> Option<&'static str> {
    match 名称 {
        "blue" => Some("蓝色"),
        _ => None,
    }
}

#[must_use]
pub fn 模式标签(模式: &str) -> Option<&'static str> {
    match 模式 {
        "dark" => Some("深色"),
        "light" => Some("浅色"),
        _ => None,
    }
}

/// 名称 ->（背景色, 浅色 standalone, 深色 standalone）；前景色统一为白色。
#[must_use]
pub fn 强调色三色(名称: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match 名称 {
        "blue" => Some(("#3584e4", "#0461be", "#81d0ff")),
        _ => None,
    }
}

/// GNOME HIG 调色板，1（最浅）到 5（最深），按序返回。
#[must_use]
pub fn 色阶(名称: &str) -> Option<[&'static str; 5]> {
    match 名称 {
        "blue" => Some(["#99c1f1", "#62a0ea", "#3584e4", "#1c71d8", "#1a5fb4"]),
        "green" => Some(["#8ff0a4", "#57e389", "#33d17a", "#2ec27e", "#26a269"]),
        "yellow" => Some(["#f9f06b", "#f8e45c", "#f6d32d", "#f5c211", "#e5a50a"]),
        "orange" => Some(["#ffbe6f", "#ffa348", "#ff7800", "#e66100", "#c64600"]),
        "red" => Some(["#f66151", "#ed333b", "#e01b24", "#c01c28", "#a51d2d"]),
        "purple" => Some(["#dc8add", "#c061cb", "#9141ac", "#813d9c", "#613583"]),
        "brown" => Some(["#cdab8f", "#b5835a", "#986a44", "#865e3c", "#63452c"]),
        "light" => Some(["#ffffff", "#f6f5f4", "#deddda", "#c0bfbc", "#9a9996"]),
        "dark" => Some(["#77767b", "#5e5c64", "#3d3846", "#241f31", "#000000"]),
        "teal" => Some(["#93ddc2", "#5bc8af", "#33b2a4", "#26a1a2", "#218787"]),
        // pink 与 slate 在 libadwaita 中只作为强调色存在，中间色阶由强调色背景
        // 与 standalone 颜色推导得到。
        "pink" => Some(["#ffa0d8", "#d56199", "#bb5587", "#a2326c", "#7a2651"]),
        "slate" => Some(["#bbd1e5", "#8c9cab", "#6f8396", "#526678", "#3e4d5a"]),
        _ => None,
    }
}

/// 由 GNOME 调色板推导的 16 个终端颜色（明暗模式相同），保持登记顺序。
#[must_use]
pub fn 终端颜色() -> [(&'static str, &'static str); 16] {
    [
        ("terminal.ansiBlack", "#241f31"),
        ("terminal.ansiRed", "#c01c28"),
        ("terminal.ansiGreen", "#26a269"),
        ("terminal.ansiYellow", "#e5a50a"),
        ("terminal.ansiBlue", "#1c71d8"),
        ("terminal.ansiMagenta", "#813d9c"),
        ("terminal.ansiCyan", "#2190a4"),
        ("terminal.ansiWhite", "#c0bfbc"),
        ("terminal.ansiBrightBlack", "#5e5c64"),
        ("terminal.ansiBrightRed", "#ed333b"),
        ("terminal.ansiBrightGreen", "#57e389"),
        ("terminal.ansiBrightYellow", "#f8e45c"),
        ("terminal.ansiBrightBlue", "#62a0ea"),
        ("terminal.ansiBrightMagenta", "#c061cb"),
        ("terminal.ansiBrightCyan", "#5bc8af"),
        ("terminal.ansiBrightWhite", "#f6f5f4"),
    ]
}

/// 单个明暗模式的基础颜色表；字段名沿用 libadwaita CSS 变量。
pub struct 基础颜色 {
    pub window_bg: &'static str,
    pub view_bg: &'static str,
    pub headerbar_bg: &'static str,
    pub headerbar_backdrop: &'static str,
    pub sidebar_bg: &'static str,
    pub sidebar_backdrop: &'static str,
    pub secondary_sidebar_bg: &'static str,
    pub secondary_sidebar_backdrop: &'static str,
    pub card_bg: &'static str,
    pub popover_bg: &'static str,
    pub dialog_bg: &'static str,
    pub overview_bg: &'static str,
    pub thumbnail_bg: &'static str,
    pub active_toggle_bg: &'static str,
    pub fg: &'static str,
    pub shade: &'static str,
    pub card_shade: &'static str,
    pub headerbar_shade: &'static str,
    pub headerbar_darker_shade: &'static str,
    pub sidebar_shade: &'static str,
    pub sidebar_border: &'static str,
    pub secondary_sidebar_shade: &'static str,
    pub secondary_sidebar_border: &'static str,
    pub popover_shade: &'static str,
    pub scrollbar_outline: &'static str,
    pub border_opacity: f64,
    pub dim_opacity: f64,
    pub disabled_opacity: f64,
    pub destructive: (&'static str, &'static str, &'static str),
    pub success: (&'static str, &'static str, &'static str),
    pub warning: (&'static str, &'static str, &'static str),
}

pub const LIGHT: 基础颜色 = 基础颜色 {
    window_bg: "#fafafb",
    view_bg: "#ffffff",
    headerbar_bg: "#ffffff",
    headerbar_backdrop: "#fafafb",
    sidebar_bg: "#ebebed",
    sidebar_backdrop: "#f2f2f4",
    secondary_sidebar_bg: "#f3f3f5",
    secondary_sidebar_backdrop: "#f6f6fa",
    card_bg: "#ffffff",
    popover_bg: "#ffffff",
    dialog_bg: "#fafafb",
    overview_bg: "#f3f3f5",
    thumbnail_bg: "#ffffff",
    active_toggle_bg: "#ffffff",
    fg: "rgb(0 0 6 / 80%)",
    shade: "rgb(0 0 6 / 7%)",
    card_shade: "rgb(0 0 6 / 7%)",
    headerbar_shade: "rgb(0 0 6 / 12%)",
    headerbar_darker_shade: "rgb(0 0 6 / 12%)",
    sidebar_shade: "rgb(0 0 6 / 7%)",
    sidebar_border: "rgb(0 0 6 / 7%)",
    secondary_sidebar_shade: "rgb(0 0 6 / 7%)",
    secondary_sidebar_border: "rgb(0 0 6 / 7%)",
    popover_shade: "rgb(0 0 6 / 7%)",
    scrollbar_outline: "#ffffff",
    border_opacity: 0.15,
    dim_opacity: 0.55,
    disabled_opacity: 0.50,
    destructive: ("#e01b24", "#ffffff", "#c30000"),
    success: ("#2ec27e", "#ffffff", "#007c3d"),
    warning: ("#e5a50a", "rgb(0 0 0 / 80%)", "#905400"),
};

pub const DARK: 基础颜色 = 基础颜色 {
    window_bg: "#222226",
    view_bg: "#1d1d20",
    headerbar_bg: "#2e2e32",
    headerbar_backdrop: "#222226",
    sidebar_bg: "#2e2e32",
    sidebar_backdrop: "#28282c",
    secondary_sidebar_bg: "#28282c",
    secondary_sidebar_backdrop: "#252529",
    card_bg: "rgb(255 255 255 / 8%)",
    popover_bg: "#36363a",
    dialog_bg: "#36363a",
    overview_bg: "#28282c",
    thumbnail_bg: "#39393d",
    active_toggle_bg: "rgb(255 255 255 / 20%)",
    fg: "#ffffff",
    shade: "rgb(0 0 6 / 25%)",
    card_shade: "rgb(0 0 6 / 36%)",
    headerbar_shade: "rgb(0 0 6 / 36%)",
    headerbar_darker_shade: "rgb(0 0 6 / 90%)",
    sidebar_shade: "rgb(0 0 6 / 25%)",
    sidebar_border: "rgb(0 0 6 / 36%)",
    secondary_sidebar_shade: "rgb(0 0 6 / 25%)",
    secondary_sidebar_border: "rgb(0 0 6 / 36%)",
    popover_shade: "rgb(0 0 6 / 25%)",
    scrollbar_outline: "rgb(0 0 6 / 50%)",
    border_opacity: 0.15,
    dim_opacity: 0.55,
    disabled_opacity: 0.50,
    destructive: ("#c01c28", "#ffffff", "#ff938c"),
    success: ("#26a269", "#ffffff", "#78e9ab"),
    warning: ("#cd9309", "rgb(0 0 0 / 80%)", "#ffc252"),
};

/// libadwaita 高对比度模式覆盖的不透明度（边框、弱化、禁用）。
pub const 高对比度不透明度: (f64, f64, f64) = (0.50, 0.90, 0.40);

/// 语法方案提供的编辑器取色；缺失项用 `None` 回退到调色板默认值。
#[derive(Default, Clone, Debug)]
pub struct 编辑器方案 {
    pub text_bg: Option<String>,
    pub text_fg: Option<String>,
    pub current_line: Option<String>,
    pub line_numbers_fg: Option<String>,
    pub line_numbers_bg: Option<String>,
    pub cursor: Option<String>,
    pub search_match_bg: Option<String>,
    pub search_match_fg: Option<String>,
    pub background_pattern: Option<String>,
}

/// 解析 `#rgb`/`#rgba`/`#rrggbb`/`#rrggbbaa`/`rgb()`，返回 `(r, g, b, alpha)`。
pub fn 解析颜色(颜色: &str) -> 结果<(u8, u8, u8, f64)> {
    let 颜色 = 颜色.trim();
    if let Some(十六进制) = 颜色.strip_prefix('#') {
        if matches!(十六进制.len(), 3 | 4 | 6 | 8)
            && 十六进制.chars().all(|字符| 字符.is_ascii_hexdigit())
        {
            let 展开: String = if 十六进制.len() <= 4 {
                十六进制.chars().flat_map(|字符| [字符, 字符]).collect()
            } else {
                十六进制.to_string()
            };
            let 分量 = |起: usize| u8::from_str_radix(&展开[起..起 + 2], 16).unwrap_or(0);
            let alpha = if 展开.len() >= 8 {
                f64::from(分量(6)) / 255.0
            } else {
                1.0
            };
            return Ok((分量(0), 分量(2), 分量(4), alpha));
        }
        return Err(工具错误::新(format!("无法解析颜色： {颜色:?}")));
    }
    match 解析rgb函数(颜色) {
        Some(值) => 值,
        None => Err(工具错误::新(format!("无法解析颜色： {颜色:?}"))),
    }
}

/// 匹配 Python 的 `rgb(0 0 6 / 36%)` 语法；不匹配返回 `None`，分量越界报错。
fn 解析rgb函数(颜色: &str) -> Option<结果<(u8, u8, u8, f64)>> {
    let 剩余 = 忽略大小写前缀(颜色, "rgb(")?;
    let 内部 = 剩余.strip_suffix(')')?;
    let 内部 = 跳空白(内部);
    let (r, 内部) = 读取数字(内部)?;
    let 内部 = 跳空白(内部);
    let (g, 内部) = 读取数字(内部)?;
    let 内部 = 跳空白(内部);
    let (b, mut 内部) = 读取数字(内部)?;
    内部 = 跳空白(内部);
    let mut alpha = 1.0;
    if let Some(尾部) = 内部.strip_prefix('/') {
        let 尾部 = 跳空白(尾部);
        let 长度 = 尾部
            .bytes()
            .take_while(|字节| 字节.is_ascii_digit() || *字节 == b'.')
            .count();
        if 长度 == 0 {
            return None;
        }
        let 文本 = &尾部[..长度];
        let 百分比 = 跳空白(&尾部[长度..]).starts_with('%');
        let 数值 = 文本.parse::<f64>().ok()?;
        alpha = if 百分比 { 数值 / 100.0 } else { 数值 };
        内部 = 跳空白(&尾部[长度..]);
        if 百分比 {
            内部 = 内部.strip_prefix('%').unwrap_or(内部);
        }
        内部 = 跳空白(内部);
    }
    if !内部.is_empty() || r > 255 || g > 255 || b > 255 || !(0.0..=1.0).contains(&alpha) {
        return Some(Err(工具错误::新(format!("颜色分量越界： {颜色:?}"))));
    }
    Some(Ok((r as u8, g as u8, b as u8, alpha)))
}

fn 读取数字(文本: &str) -> Option<(u64, &str)> {
    let 长度 = 文本.bytes().take_while(u8::is_ascii_digit).count();
    if 长度 == 0 {
        None
    } else {
        Some((文本[..长度].parse().ok()?, &文本[长度..]))
    }
}

fn 跳空白(文本: &str) -> &str {
    文本.trim_start_matches(char::is_whitespace)
}

fn 忽略大小写前缀<'a>(文本: &'a str, 前缀: &str) -> Option<&'a str> {
    if 文本
        .get(..前缀.len())
        .is_some_and(|开头| 开头.eq_ignore_ascii_case(前缀))
    {
        Some(&文本[前缀.len()..])
    } else {
        None
    }
}

/// 按 Python `round()` 的银行家舍入取整后转为十六进制；alpha 达 1 时省略。
#[must_use]
pub fn 转为十六进制(r: f64, g: f64, b: f64, alpha: f64) -> String {
    let 取整 = |值: f64| 值.round_ties_even().clamp(0.0, 255.0) as u8;
    if alpha >= 1.0 {
        format!("#{:02x}{:02x}{:02x}", 取整(r), 取整(g), 取整(b))
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            取整(r),
            取整(g),
            取整(b),
            取整(alpha * 255.0)
        )
    }
}

fn 校验比例(比例: f64) -> 结果<()> {
    if 比例.is_finite() && (0.0..=1.0).contains(&比例) {
        Ok(())
    } else {
        Err(工具错误::新("颜色比例必须是 0 至 1 之间的有限数值"))
    }
}

/// 在颜色原有的透明度上乘以 `alpha`。
pub fn 合成透明颜色(颜色: &str, alpha: f64) -> 结果<String> {
    校验比例(alpha)?;
    let (r, g, b, 原alpha) = 解析颜色(颜色)?;
    Ok(转为十六进制(
        f64::from(r),
        f64::from(g),
        f64::from(b),
        alpha * 原alpha,
    ))
}

/// 规范化为 `#rrggbb` / `#rrggbbaa`；VS Code 主题解析器只接受十六进制。
pub fn 规范颜色格式(颜色: &str) -> 结果<String> {
    let (r, g, b, alpha) = 解析颜色(颜色)?;
    Ok(转为十六进制(
        f64::from(r),
        f64::from(g),
        f64::from(b),
        alpha,
    ))
}

/// 把 `weight` 份的 `color_a` 混入 `color_b`（两者都必须不透明）。
pub fn 混色(色a: &str, 色b: &str, weight: f64) -> 结果<String> {
    校验比例(weight)?;
    let (ra, ga, ba, aa) = 解析颜色(色a)?;
    let (rb, gb, bb, ab) = 解析颜色(色b)?;
    if aa < 1.0 || ab < 1.0 {
        return Err(工具错误::新("mix() 要求颜色不透明"));
    }
    Ok(转为十六进制(
        f64::from(ra) * weight + f64::from(rb) * (1.0 - weight),
        f64::from(ga) * weight + f64::from(gb) * (1.0 - weight),
        f64::from(ba) * weight + f64::from(bb) * (1.0 - weight),
        1.0,
    ))
}

/// 把（可能半透明的）颜色合成到不透明背景之上。
pub fn 叠加颜色(颜色: &str, 背景: &str) -> 结果<String> {
    let (r, g, b, alpha) = 解析颜色(颜色)?;
    let (br, bg, bb, 背景透明度) = 解析颜色(背景)?;
    if 背景透明度 < 1.0 {
        return Err(工具错误::新("叠加颜色要求背景不透明"));
    }
    if alpha >= 1.0 {
        return Ok(转为十六进制(
            f64::from(r),
            f64::from(g),
            f64::from(b),
            1.0,
        ));
    }
    Ok(转为十六进制(
        f64::from(r) * alpha + f64::from(br) * (1.0 - alpha),
        f64::from(g) * alpha + f64::from(bg) * (1.0 - alpha),
        f64::from(b) * alpha + f64::from(bb) * (1.0 - alpha),
        1.0,
    ))
}

/// 某一（模式, 强调色, 对比度）组合的具体颜色。
pub struct 调色板对象 {
    pub mode: String,
    pub accent: String,
    pub high_contrast: bool,
    pub scheme: 编辑器方案,
    角色: 有序映射,
}

impl 调色板对象 {
    pub fn 新(
        模式: &str, 强调色: &str, high_contrast: bool, scheme: 编辑器方案
    ) -> 结果<Self> {
        if !matches!(模式, "dark" | "light") {
            return Err(工具错误::新(format!("未知主题模式： {模式:?}")));
        }
        if 强调色三色(强调色).is_none() {
            return Err(工具错误::新(format!("未知强调色： {强调色:?}")));
        }
        let dark = 模式 == "dark";
        let base = if dark { &DARK } else { &LIGHT };
        let (border_opacity, dim_opacity, disabled_opacity) = if high_contrast {
            高对比度不透明度
        } else {
            (base.border_opacity, base.dim_opacity, base.disabled_opacity)
        };
        let fg_raw = if high_contrast && !dark {
            "rgb(0 0 6 / 100%)"
        } else {
            base.fg
        };
        let window = base.window_bg;
        let view = base.view_bg;
        let sidebar = base.sidebar_bg;
        let headerbar = base.headerbar_bg;
        let (fg_r, fg_g, fg_b, _) = 解析颜色(fg_raw)?;
        let fg = 转为十六进制(f64::from(fg_r), f64::from(fg_g), f64::from(fg_b), 1.0);

        let mut c = 有序映射::新();
        // 表面
        c.放("bg_window", window);
        c.放("bg_view", base.view_bg);
        c.放("bg_headerbar", base.headerbar_bg);
        c.放("bg_headerbar_backdrop", base.headerbar_backdrop);
        c.放("bg_sidebar", base.sidebar_bg);
        c.放("bg_sidebar_backdrop", base.sidebar_backdrop);
        c.放("bg_sidebar_secondary", base.secondary_sidebar_bg);
        c.放(
            "bg_sidebar_secondary_backdrop",
            base.secondary_sidebar_backdrop,
        );
        let bg_card = 叠加颜色(base.card_bg, window)?;
        c.放("bg_card", bg_card.clone());
        c.放("bg_popover", base.popover_bg);
        c.放("bg_dialog", base.dialog_bg);
        c.放("bg_overview", base.overview_bg);
        c.放("bg_thumbnail", base.thumbnail_bg);

        // 文本
        c.放("fg", 叠加颜色(fg_raw, window)?);
        c.放("fg_window", 叠加颜色(fg_raw, window)?);
        c.放("fg_view", 叠加颜色(fg_raw, view)?);
        c.放("fg_headerbar", 叠加颜色(fg_raw, headerbar)?);
        c.放("fg_sidebar", 叠加颜色(fg_raw, sidebar)?);
        c.放(
            "fg_sidebar_secondary",
            叠加颜色(fg_raw, base.secondary_sidebar_bg)?,
        );
        c.放("fg_card", 叠加颜色(fg_raw, &bg_card)?);
        c.放("fg_popover", 叠加颜色(fg_raw, base.popover_bg)?);

        // 交互表面（libadwaita 的按钮/列表行使用 currentColor 的 7-15%）
        let 表面: [(&str, &str); 6] = [
            ("window", window),
            ("view", view),
            ("sidebar", sidebar),
            ("headerbar", headerbar),
            ("popover", base.popover_bg),
            ("card", bg_card.as_str()),
        ];
        for (名称, 背景) in 表面 {
            c.放(
                format!("bg_hover_{名称}"),
                叠加颜色(&合成透明颜色(fg_raw, 0.07)?, 背景)?,
            );
            c.放(
                format!("bg_active_{名称}"),
                叠加颜色(&合成透明颜色(fg_raw, 0.12)?, 背景)?,
            );
            c.放(
                format!("fg_dim_{名称}"),
                叠加颜色(&合成透明颜色(fg_raw, dim_opacity)?, 背景)?,
            );
            c.放(
                format!("fg_disabled_{名称}"),
                叠加颜色(&合成透明颜色(fg_raw, disabled_opacity)?, 背景)?,
            );
        }
        c.放("bg_hover", c.取拷贝("bg_hover_window"));
        c.放("bg_active", c.取拷贝("bg_active_window"));
        c.放("fg_dim", c.取拷贝("fg_dim_window"));
        c.放("fg_disabled", c.取拷贝("fg_disabled_window"));
        c.放(
            "fg_placeholder",
            叠加颜色(&合成透明颜色(fg_raw, disabled_opacity)?, view)?,
        );
        c.放("bg_button", 叠加颜色(&合成透明颜色(fg_raw, 0.10)?, window)?);
        c.放(
            "bg_button_headerbar",
            叠加颜色(&合成透明颜色(fg_raw, 0.10)?, headerbar)?,
        );
        c.放(
            "bg_button_view",
            叠加颜色(&合成透明颜色(fg_raw, 0.10)?, view)?,
        );
        c.放(
            "bg_button_sidebar",
            叠加颜色(&合成透明颜色(fg_raw, 0.10)?, sidebar)?,
        );
        c.放(
            "bg_button_hover",
            叠加颜色(&合成透明颜色(fg_raw, 0.13)?, window)?,
        );
        c.放(
            "bg_button_active",
            叠加颜色(&合成透明颜色(fg_raw, 0.16)?, window)?,
        );
        let bg_input = if !dark {
            view.to_string()
        } else {
            叠加颜色(&合成透明颜色(fg_raw, 0.07)?, view)?
        };
        c.放("bg_input", bg_input.clone());
        c.放(
            "bg_input_hover",
            叠加颜色(&合成透明颜色(fg_raw, 0.10)?, &bg_input)?,
        );
        c.放(
            "bg_input_disabled",
            叠加颜色(&合成透明颜色(fg_raw, 0.05)?, &bg_input)?,
        );

        // 边框
        c.放("border", 合成透明颜色(fg_raw, border_opacity)?);
        c.放(
            "border_strong",
            合成透明颜色(fg_raw, border_opacity.max(0.5))?,
        );
        c.放("border_dim", 合成透明颜色(fg_raw, border_opacity * 0.6)?);
        c.放(
            "border_input",
            合成透明颜色(fg_raw, border_opacity.max(0.25))?,
        );
        let (accent_bg, accent_standalone_light, accent_standalone_dark) =
            强调色三色(强调色).expect("强调色已校验");
        c.放(
            "border_input_focus",
            合成透明颜色(accent_bg, if high_contrast { 1.0 } else { 0.5 })?,
        );
        c.放("border_tab", 合成透明颜色(fg_raw, border_opacity)?);
        c.放("contrast_border", fg.clone());

        // 阴影色（转为十六进制：VS Code 只接受 #rrggbb / #rrggbbaa）
        for 名称 in [
            "shade",
            "card_shade",
            "headerbar_shade",
            "headerbar_darker_shade",
            "sidebar_shade",
            "sidebar_border",
            "secondary_sidebar_shade",
            "secondary_sidebar_border",
            "popover_shade",
        ] {
            let 原值 = 取基础色(base, 名称);
            c.放(名称, 规范颜色格式(原值)?);
        }
        c.放("scrollbar_outline", 规范颜色格式(base.scrollbar_outline)?);

        // 滚动条：currentColor 20%（悬停 60%，激活 100%，对应 Adwaita 悬浮滚动条）
        c.放(
            "scrollbar",
            合成透明颜色(fg_raw, if high_contrast { 0.40 } else { 0.20 })?,
        );
        c.放("scrollbar_hover", 合成透明颜色(fg_raw, 0.60)?);
        c.放("scrollbar_active", fg.clone());

        // 强调色
        c.放("accent_bg", accent_bg);
        // 蓝色强调色前景采用 #ffffff。
        c.放("accent_fg", "#ffffff");
        c.放(
            "accent_standalone",
            if dark {
                accent_standalone_dark
            } else {
                accent_standalone_light
            },
        );
        // mix 的权重属于第一个颜色；悬停及按下仍以强调色为主体。
        c.放(
            "accent_hover",
            if dark {
                混色(accent_bg, "#ffffff", 0.85)?
            } else {
                混色(accent_bg, "#000000", 0.88)?
            },
        );
        c.放(
            "accent_active",
            if dark {
                混色(accent_bg, "#ffffff", 0.75)?
            } else {
                混色(accent_bg, "#000000", 0.80)?
            },
        );
        for (名称, 背景) in [("view", view), ("window", window), ("sidebar", sidebar)] {
            c.放(
                format!("accent_soft_{名称}"),
                叠加颜色(&合成透明颜色(accent_bg, 0.25)?, 背景)?,
            );
            c.放(
                format!("accent_faint_{名称}"),
                叠加颜色(&合成透明颜色(accent_bg, 0.15)?, 背景)?,
            );
        }
        c.放("accent_soft", c.取拷贝("accent_soft_view"));
        c.放("accent_faint", c.取拷贝("accent_faint_view"));

        // 状态色
        for (名称, (背景, 前景, standalone)) in [
            ("destructive", base.destructive),
            ("success", base.success),
            ("warning", base.warning),
        ] {
            c.放(format!("{名称}_bg"), 背景);
            c.放(format!("{名称}_fg"), 叠加颜色(前景, 背景)?);
            c.放(format!("{名称}_standalone"), standalone);
        }
        c.放("error_bg", c.取拷贝("destructive_bg"));
        c.放("error_fg", c.取拷贝("destructive_fg"));
        c.放("error_standalone", c.取拷贝("destructive_standalone"));

        // 编辑器
        let 取值 = |字段: &Option<String>| 字段.clone();
        c.放(
            "editor_bg",
            取值(&scheme.text_bg).unwrap_or_else(|| view.to_string()),
        );
        c.放(
            "editor_fg",
            取值(&scheme.text_fg).unwrap_or_else(|| c.取拷贝("fg_view").to_string()),
        );
        c.放(
            "editor_line_highlight",
            取值(&scheme.current_line).unwrap_or(叠加颜色(&合成透明颜色(fg_raw, 0.05)?, view)?),
        );
        c.放(
            "editor_line_number",
            取值(&scheme.line_numbers_fg).unwrap_or_else(|| c.取拷贝("fg_dim_view").to_string()),
        );
        c.放(
            "editor_line_number_bg",
            取值(&scheme.line_numbers_bg).unwrap_or_else(|| c.取拷贝("editor_bg").to_string()),
        );
        c.放(
            "editor_cursor",
            取值(&scheme.cursor).unwrap_or_else(|| c.取拷贝("accent_standalone").to_string()),
        );
        c.放(
            "editor_search_match",
            取值(&scheme.search_match_bg).unwrap_or_else(|| c.取拷贝("accent_soft").to_string()),
        );
        c.放(
            "editor_search_match_fg",
            取值(&scheme.search_match_fg).unwrap_or_else(|| c.取拷贝("editor_fg").to_string()),
        );
        c.放(
            "editor_background_pattern",
            取值(&scheme.background_pattern).unwrap_or_else(|| c.取拷贝("editor_bg").to_string()),
        );

        // 由强调色推导的编辑器交互色
        let editor_bg = c.取拷贝("editor_bg").to_string();
        c.放(
            "selection",
            叠加颜色(&合成透明颜色(accent_bg, 0.25)?, &editor_bg)?,
        );
        c.放(
            "selection_inactive",
            叠加颜色(&合成透明颜色(accent_bg, 0.15)?, &editor_bg)?,
        );
        c.放(
            "selection_highlight",
            叠加颜色(&合成透明颜色(accent_bg, 0.15)?, &editor_bg)?,
        );
        c.放(
            "word_highlight",
            叠加颜色(&合成透明颜色(accent_bg, 0.20)?, &editor_bg)?,
        );
        c.放(
            "word_highlight_strong",
            叠加颜色(&合成透明颜色(accent_bg, 0.30)?, &editor_bg)?,
        );
        c.放(
            "find_match",
            叠加颜色(&合成透明颜色(accent_bg, 0.35)?, &editor_bg)?,
        );
        c.放(
            "find_match_highlight",
            叠加颜色(&合成透明颜色(accent_bg, 0.20)?, &editor_bg)?,
        );
        c.放(
            "bracket_match",
            叠加颜色(&合成透明颜色(accent_bg, 0.30)?, &editor_bg)?,
        );
        c.放("bracket_border", c.取拷贝("accent_standalone"));
        c.放(
            "indent_guide",
            合成透明颜色(fg_raw, border_opacity.max(0.15))?,
        );
        c.放("indent_guide_active", 合成透明颜色(fg_raw, 0.30)?);
        c.放("editor_ruler", 合成透明颜色(fg_raw, 0.15)?);

        // 差异 / 合并
        let success_standalone = c.取拷贝("success_standalone");
        let destructive_standalone = c.取拷贝("destructive_standalone");
        c.放(
            "diff_inserted",
            叠加颜色(&合成透明颜色(&success_standalone, 0.35)?, &editor_bg)?,
        );
        c.放(
            "diff_removed",
            叠加颜色(&合成透明颜色(&destructive_standalone, 0.35)?, &editor_bg)?,
        );
        c.放(
            "diff_inserted_bg",
            叠加颜色(&合成透明颜色(&success_standalone, 0.15)?, &editor_bg)?,
        );
        c.放(
            "diff_removed_bg",
            叠加颜色(&合成透明颜色(&destructive_standalone, 0.15)?, &editor_bg)?,
        );
        c.放("diff_base", editor_bg);

        Ok(Self {
            mode: 模式.to_string(),
            accent: 强调色.to_string(),
            high_contrast,
            scheme,
            角色: c,
        })
    }

    #[must_use]
    pub fn 取(&self, 角色: &str) -> &str {
        self.角色.必须取(角色)
    }

    #[must_use]
    pub fn 取可选(&self, 角色: &str) -> Option<&str> {
        self.角色.取(角色)
    }

    #[must_use]
    pub fn 角色表(&self) -> &indexmap::map::Slice<String, String> {
        self.角色.条目()
    }

    /// 把颜色的 `权重` 比例合成到具名表面之上。
    pub fn 表面叠加(&self, 颜色: &str, 权重: f64, 表面: &str) -> 结果<String> {
        叠加颜色(&合成透明颜色(颜色, 权重)?, self.取(表面))
    }
}

fn 取基础色(base: &基础颜色, 名称: &str) -> &'static str {
    match 名称 {
        "shade" => base.shade,
        "card_shade" => base.card_shade,
        "headerbar_shade" => base.headerbar_shade,
        "headerbar_darker_shade" => base.headerbar_darker_shade,
        "sidebar_shade" => base.sidebar_shade,
        "sidebar_border" => base.sidebar_border,
        "secondary_sidebar_shade" => base.secondary_sidebar_shade,
        "secondary_sidebar_border" => base.secondary_sidebar_border,
        "popover_shade" => base.popover_shade,
        _ => unreachable!("未知基础色： {名称}"),
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 解析与格式化颜色() {
        assert_eq!(解析颜色("#abc").unwrap(), (170, 187, 204, 1.0));
        let (r, g, b, a) = 解析颜色("#abcd").unwrap();
        assert_eq!((r, g, b), (170, 187, 204));
        assert!((a - 0.866_666_666_666_666_7).abs() < 1e-12);
        assert_eq!(解析颜色("rgb(0 0 6 / 36%)").unwrap(), (0, 0, 6, 0.36));
        assert_eq!(解析颜色("rgb(0 0 6 / 0.5)").unwrap(), (0, 0, 6, 0.5));
        assert_eq!(解析颜色("rgb(1 2 3)").unwrap(), (1, 2, 3, 1.0));
        assert_eq!(解析颜色("rgb(0 0 6/7%)").unwrap(), (0, 0, 6, 0.07));
        assert!(解析颜色("#12").is_err());
        assert!(解析颜色("rgb(300 0 0)").is_err());
        assert_eq!(转为十六进制(1.5, 2.5, 3.5, 1.0), "#020204");
        assert_eq!(转为十六进制(0.0, 0.0, 6.0, 0.36), "#0000065c");
        assert_eq!(合成透明颜色("rgb(0 0 6 / 80%)", 0.07).unwrap(), "#0000060e");
        assert_eq!(规范颜色格式("rgb(0 0 6 / 7%)").unwrap(), "#00000612");
        assert_eq!(混色("#3584e4", "#ffffff", 0.85).unwrap(), "#5396e8");
        assert_eq!(混色("#3584e4", "#000000", 0.88).unwrap(), "#2f74c9");
        assert_eq!(叠加颜色("rgb(0 0 6 / 80%)", "#fafafb").unwrap(), "#323237");
    }

    #[test]
    fn 角色与金样例一致() {
        let 路径 = concat!(env!("CARGO_MANIFEST_DIR"), "/测试/Rust/调色板样例.json");
        let 文本 = std::fs::read_to_string(路径).expect("读取调色板金样例");
        let 样例: serde_json::Value = serde_json::from_str(&文本).expect("解析调色板金样例");
        for (场景, 期望) in 样例.as_object().expect("样例为对象") {
            let (模式, high_contrast) = match 场景.as_str() {
                "dark-普通" => ("dark", false),
                "dark-高对比度" => ("dark", true),
                "light-普通" => ("light", false),
                "light-高对比度" => ("light", true),
                其他 => panic!("未知场景：{其他}"),
            };
            let 调色板 = 调色板对象::新(模式, "blue", high_contrast, 编辑器方案::default())
                .expect("构建调色板");
            let 期望表 = 期望.as_object().expect("场景为对象");
            assert_eq!(调色板.角色表().len(), 期望表.len(), "场景 {场景} 角色数量");
            for (角色, 期望值) in 期望表 {
                assert_eq!(
                    调色板.取可选(角色),
                    Some(期望值.as_str().expect("颜色为字符串")),
                    "场景 {场景} 角色 {角色}"
                );
            }
        }
    }

    #[test]
    fn 校验参数() {
        assert!(调色板对象::新("综合", "blue", false, 编辑器方案::default()).is_err());
        assert!(调色板对象::新("dark", "紫色", false, 编辑器方案::default()).is_err());
    }
    #[test]
    fn 颜色运算拒绝非法比例及半透明背景() {
        for 比例 in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert!(合成透明颜色("#fff", 比例).is_err());
            assert!(混色("#fff", "#000", 比例).is_err());
        }
        assert!(叠加颜色("#fff", "#0000").is_err());
    }
}
