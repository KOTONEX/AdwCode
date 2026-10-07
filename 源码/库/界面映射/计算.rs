// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 固定的具名颜色计算；由 Rust 编译检查，不解析表达式或执行外部代码。
use super::*;
type 颜色计算 = fn(&上下文<'_>) -> 结果<Option<String>>;
fn 半透明(上下文: &上下文<'_>, 角色: &str, 比例: f64) -> 结果<String> {
    合成透明颜色(&角色值(上下文.调色板, 角色)?, 比例)
}
fn 叠加背景(
    上下文: &上下文<'_>, 角色: &str, 比例: f64, 表面: &str
) -> 结果<String> {
    叠加颜色(&半透明(上下文, 角色, 比例)?, &角色值(上下文.调色板, 表面)?)
}
fn 选取色阶(上下文: &上下文<'_>, 名称: &str, 深: usize, 浅: usize) -> 结果<String> {
    let 级 = if 上下文.调色板.mode == "dark" {
        深
    } else {
        浅
    };
    色阶(名称)
        .and_then(|色阶| 色阶.get(级 - 1).copied())
        .map(str::to_owned)
        .ok_or_else(|| 工具错误::新("未知色阶或级数"))
}
fn 样式前景(
    上下文: &上下文<'_>,
    名称: &str,
    回退: impl FnOnce() -> 结果<String>,
) -> 结果<String> {
    match 上下文
        .样式
        .get(名称)
        .and_then(|样式| 样式.foreground.as_deref())
        .filter(|值| !值.is_empty())
    {
        Some(值) => Ok(值.to_owned()),
        None => 回退(),
    }
}
// 名称与函数指针对应；重复引用共享计算定义，顺序由映射表控制。
const 计算表: &[(&str, 颜色计算)] = &[
    ("半透明_accent_bg_75", |上下文| {
        半透明(上下文, "accent_bg", 0.75).map(Some)
    }),
    ("高对比度_contrast_border_否则_省略", |上下文| {
        Ok(if 上下文.调色板.high_contrast {
            Some(角色值(上下文.调色板, "contrast_border")?)
        } else {
            None
        })
    }),
    (
        "高对比度_半透明_accent_bg_75_否则_省略",
        |上下文| {
            Ok(if 上下文.调色板.high_contrast {
                Some(合成透明颜色(
                    &(角色值(上下文.调色板, "accent_bg")?),
                    0.75,
                )?)
            } else {
                None
            })
        },
    ),
    ("深色_颜色00000066_否则_颜色00000026", |上下文| {
        Ok(if 上下文.调色板.mode == "dark" {
            Some("#00000066".to_owned())
        } else {
            Some("#00000026".to_owned())
        })
    }),
    ("半透明_accent_bg_35", |上下文| {
        半透明(上下文, "accent_bg", 0.35).map(Some)
    }),
    ("色阶_teal_4_5", |上下文| {
        选取色阶(上下文, "teal", 4, 5).map(Some)
    }),
    (
        "高对比度_sidebar_border_否则_颜色00000000",
        |上下文| {
            Ok(if 上下文.调色板.high_contrast {
                Some(角色值(上下文.调色板, "sidebar_border")?)
            } else {
                Some("#00000000".to_owned())
            })
        },
    ),
    ("半透明_accent_bg_12", |上下文| {
        半透明(上下文, "accent_bg", 0.12).map(Some)
    }),
    ("深色_warning_fg_否则_accent_fg", |上下文| {
        Ok(if 上下文.调色板.mode == "dark" {
            Some(角色值(上下文.调色板, "warning_fg")?)
        } else {
            Some(角色值(上下文.调色板, "accent_fg")?)
        })
    }),
    ("色阶_blue_2_4", |上下文| {
        选取色阶(上下文, "blue", 2, 4).map(Some)
    }),
    ("色阶_orange_2_4", |上下文| {
        选取色阶(上下文, "orange", 2, 4).map(Some)
    }),
    ("色阶_purple_1_4", |上下文| {
        选取色阶(上下文, "purple", 1, 4).map(Some)
    }),
    ("色阶_green_2_5", |上下文| {
        选取色阶(上下文, "green", 2, 5).map(Some)
    }),
    ("色阶_pink_2_5", |上下文| {
        选取色阶(上下文, "pink", 2, 5).map(Some)
    }),
    ("色阶_teal_2_5", |上下文| {
        选取色阶(上下文, "teal", 2, 5).map(Some)
    }),
    ("半透明_error_bg_12", |上下文| {
        半透明(上下文, "error_bg", 0.12).map(Some)
    }),
    ("半透明_warning_bg_12", |上下文| {
        半透明(上下文, "warning_bg", 0.12).map(Some)
    }),
    ("半透明_accent_bg_20", |上下文| {
        半透明(上下文, "accent_bg", 0.2).map(Some)
    }),
    (
        "浅色标签_叠加_fg_6_bg_window_否则_bg_headerbar",
        |上下文| {
            Ok(
                if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                    Some(叠加颜色(
                        &合成透明颜色(&(角色值(上下文.调色板, "fg")?), 0.06)?,
                        &角色值(上下文.调色板, "bg_window")?,
                    )?)
                } else {
                    Some(角色值(上下文.调色板, "bg_headerbar")?)
                },
            )
        },
    ),
    ("浅色标签_border_否则_颜色00000000", |上下文| {
        Ok(
            if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                Some(角色值(上下文.调色板, "border")?)
            } else {
                Some("#00000000".to_owned())
            },
        )
    }),
    (
        "浅色标签_bg_view_否则_叠加_fg_10_bg_headerbar",
        |上下文| {
            Ok(
                if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                    Some(角色值(上下文.调色板, "bg_view")?)
                } else {
                    Some(叠加颜色(
                        &合成透明颜色(&(角色值(上下文.调色板, "fg")?), 0.1)?,
                        &角色值(上下文.调色板, "bg_headerbar")?,
                    )?)
                },
            )
        },
    ),
    (
        "浅色标签_浅色标签_bg_view_否则_叠加_fg_10_bg_headerbar_否则_叠加_fg_6_bg_headerbar_backdrop",
        |上下文| {
            Ok(
                if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                    if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                        Some(角色值(上下文.调色板, "bg_view")?)
                    } else {
                        Some(叠加颜色(
                            &合成透明颜色(&(角色值(上下文.调色板, "fg")?), 0.1)?,
                            &角色值(上下文.调色板, "bg_headerbar")?,
                        )?)
                    }
                } else {
                    Some(叠加颜色(
                        &合成透明颜色(&(角色值(上下文.调色板, "fg")?), 0.06)?,
                        &角色值(上下文.调色板, "bg_headerbar_backdrop")?,
                    )?)
                },
            )
        },
    ),
    (
        "浅色标签_叠加_fg_75_bg_window_否则_fg_dim_headerbar",
        |上下文| {
            Ok(
                if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                    Some(叠加颜色(
                        &合成透明颜色(&(角色值(上下文.调色板, "fg")?), 0.75)?,
                        &角色值(上下文.调色板, "bg_window")?,
                    )?)
                } else {
                    Some(角色值(上下文.调色板, "fg_dim_headerbar")?)
                },
            )
        },
    ),
    (
        "浅色标签_浅色标签_叠加_fg_6_bg_window_否则_bg_headerbar_否则_bg_headerbar_backdrop",
        |上下文| {
            Ok(
                if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                    if 上下文.调色板.mode != "dark" && !上下文.调色板.high_contrast {
                        Some(叠加颜色(
                            &合成透明颜色(&(角色值(上下文.调色板, "fg")?), 0.06)?,
                            &角色值(上下文.调色板, "bg_window")?,
                        )?)
                    } else {
                        Some(角色值(上下文.调色板, "bg_headerbar")?)
                    }
                } else {
                    Some(角色值(上下文.调色板, "bg_headerbar_backdrop")?)
                },
            )
        },
    ),
    ("叠加_accent_bg_25_bg_popover", |上下文| {
        叠加背景(上下文, "accent_bg", 0.25, "bg_popover").map(Some)
    }),
    ("半透明_success_bg_20", |上下文| {
        半透明(上下文, "success_bg", 0.2).map(Some)
    }),
    ("半透明_error_bg_20", |上下文| {
        半透明(上下文, "error_bg", 0.2).map(Some)
    }),
    ("叠加_accent_bg_25_bg_sidebar", |上下文| {
        叠加背景(上下文, "accent_bg", 0.25, "bg_sidebar").map(Some)
    }),
    ("叠加_fg_popover_10_bg_popover", |上下文| {
        叠加背景(上下文, "fg_popover", 0.1, "bg_popover").map(Some)
    }),
    ("叠加_error_bg_15_bg_input", |上下文| {
        叠加背景(上下文, "error_bg", 0.15, "bg_input").map(Some)
    }),
    ("叠加_warning_bg_15_bg_input", |上下文| {
        叠加背景(上下文, "warning_bg", 0.15, "bg_input").map(Some)
    }),
    ("叠加_accent_bg_15_bg_input", |上下文| {
        叠加背景(上下文, "accent_bg", 0.15, "bg_input").map(Some)
    }),
    ("高对比度_border_否则_border_dim", |上下文| {
        Ok(if 上下文.调色板.high_contrast {
            Some(角色值(上下文.调色板, "border")?)
        } else {
            Some(角色值(上下文.调色板, "border_dim")?)
        })
    }),
    ("样式_def:identifier_回退_fg", |上下文| {
        样式前景(上下文, "def:identifier", || {
            角色值(上下文.调色板, "fg")
        })
        .map(Some)
    }),
    ("样式_def:string_回退_fg", |上下文| {
        样式前景(上下文, "def:string", || 角色值(上下文.调色板, "fg")).map(Some)
    }),
    ("样式_def:boolean_回退_fg", |上下文| {
        样式前景(上下文, "def:boolean", || 角色值(上下文.调色板, "fg")).map(Some)
    }),
    ("样式_def:number_回退_fg", |上下文| {
        样式前景(上下文, "def:number", || 角色值(上下文.调色板, "fg")).map(Some)
    }),
    ("半透明_success_bg_25", |上下文| {
        半透明(上下文, "success_bg", 0.25).map(Some)
    }),
    ("半透明_success_bg_40", |上下文| {
        半透明(上下文, "success_bg", 0.4).map(Some)
    }),
    ("半透明_fg_dim_25", |上下文| {
        半透明(上下文, "fg_dim", 0.25).map(Some)
    }),
    ("半透明_fg_dim_40", |上下文| {
        半透明(上下文, "fg_dim", 0.4).map(Some)
    }),
    ("半透明_accent_bg_10", |上下文| {
        半透明(上下文, "accent_bg", 0.1).map(Some)
    }),
    ("半透明_error_bg_25", |上下文| {
        半透明(上下文, "error_bg", 0.25).map(Some)
    }),
    ("半透明_accent_bg_15", |上下文| {
        半透明(上下文, "accent_bg", 0.15).map(Some)
    }),
    ("半透明_accent_bg_25", |上下文| {
        半透明(上下文, "accent_bg", 0.25).map(Some)
    }),
    ("半透明_accent_bg_30", |上下文| {
        半透明(上下文, "accent_bg", 0.3).map(Some)
    }),
    ("样式_def:type_回退_fg", |上下文| {
        样式前景(上下文, "def:type", || 角色值(上下文.调色板, "fg")).map(Some)
    }),
    ("色阶_red_2_4", |上下文| {
        选取色阶(上下文, "red", 2, 4).map(Some)
    }),
    ("样式_def:constant_回退_fg", |上下文| {
        样式前景(上下文, "def:constant", || 角色值(上下文.调色板, "fg")).map(Some)
    }),
    ("样式_def:statement_回退_fg", |上下文| {
        样式前景(上下文, "def:statement", || {
            角色值(上下文.调色板, "fg")
        })
        .map(Some)
    }),
    ("色阶_red_1_4", |上下文| {
        选取色阶(上下文, "red", 1, 4).map(Some)
    }),
    ("色阶_blue_1_4", |上下文| {
        选取色阶(上下文, "blue", 1, 4).map(Some)
    }),
    ("色阶_yellow_4_5", |上下文| {
        选取色阶(上下文, "yellow", 4, 5).map(Some)
    }),
    ("色阶_orange_1_4", |上下文| {
        选取色阶(上下文, "orange", 1, 4).map(Some)
    }),
    ("色阶_green_1_5", |上下文| {
        选取色阶(上下文, "green", 1, 5).map(Some)
    }),
    ("半透明_error_bg_40", |上下文| {
        半透明(上下文, "error_bg", 0.4).map(Some)
    }),
    ("半透明_warning_bg_25", |上下文| {
        半透明(上下文, "warning_bg", 0.25).map(Some)
    }),
    ("叠加_fg_10_bg_headerbar", |上下文| {
        叠加背景(上下文, "fg", 0.1, "bg_headerbar").map(Some)
    }),
    ("叠加_fg_13_bg_headerbar", |上下文| {
        叠加背景(上下文, "fg", 0.13, "bg_headerbar").map(Some)
    }),
    ("叠加_fg_16_bg_headerbar", |上下文| {
        叠加背景(上下文, "fg", 0.16, "bg_headerbar").map(Some)
    }),
    ("半透明_色阶_blue_2_4_35", |上下文| {
        Ok(Some(合成透明颜色(
            &(选取色阶(上下文, "blue", 2, 4)?),
            0.35,
        )?))
    }),
    ("半透明_色阶_orange_2_4_35", |上下文| {
        Ok(Some(合成透明颜色(
            &(选取色阶(上下文, "orange", 2, 4)?),
            0.35,
        )?))
    }),
    ("半透明_色阶_purple_1_4_35", |上下文| {
        Ok(Some(合成透明颜色(
            &(选取色阶(上下文, "purple", 1, 4)?),
            0.35,
        )?))
    }),
    ("半透明_色阶_green_2_5_35", |上下文| {
        Ok(Some(合成透明颜色(
            &(选取色阶(上下文, "green", 2, 5)?),
            0.35,
        )?))
    }),
    ("半透明_色阶_pink_2_5_35", |上下文| {
        Ok(Some(合成透明颜色(
            &(选取色阶(上下文, "pink", 2, 5)?),
            0.35,
        )?))
    }),
    ("半透明_色阶_teal_2_5_35", |上下文| {
        Ok(Some(合成透明颜色(
            &(选取色阶(上下文, "teal", 2, 5)?),
            0.35,
        )?))
    }),
    ("半透明_accent_bg_40", |上下文| {
        半透明(上下文, "accent_bg", 0.4).map(Some)
    }),
    ("半透明_accent_bg_8", |上下文| {
        半透明(上下文, "accent_bg", 0.08).map(Some)
    }),
    ("色阶_pink_2_4", |上下文| {
        选取色阶(上下文, "pink", 2, 4).map(Some)
    }),
    ("半透明_fg_view_4", |上下文| {
        半透明(上下文, "fg_view", 0.04).map(Some)
    }),
    ("半透明_warning_bg_20", |上下文| {
        半透明(上下文, "warning_bg", 0.2).map(Some)
    }),
    ("高对比度_border_否则_颜色00000000", |上下文| {
        Ok(if 上下文.调色板.high_contrast {
            Some(角色值(上下文.调色板, "border")?)
        } else {
            Some("#00000000".to_owned())
        })
    }),
];
pub(super) fn 查找计算(名称: &str) -> 结果<颜色计算> {
    计算表
        .iter()
        .find(|(已有, _)| *已有 == 名称)
        .map(|(_, 函数)| *函数)
        .ok_or_else(|| 工具错误::新(format!("未知具名颜色计算：{名称}")))
}
