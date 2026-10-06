// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! VS Code 工作台颜色到 Adwaita 角色的映射。
//!
//! 映射遵循 libadwaita 1.10 的约定：
//!
//! - 窗口 / 标题栏 / 侧栏 / 视图各表面保留各自色阶；
//! - 列表选中使用 25% 强调色（对应 `--view-selected-color`）；
//! - 悬停与“激活”背景分别为前景色的 7% / 12%；
//! - 边框为 `currentColor` 的 15%（高对比度为 50%）；
//! - 标签页对齐 `AdwTabBar`：浅色采用浅灰条带与白色活动标签，深色采用 10% 高亮。

use std::collections::BTreeMap;
use std::path::Path;

use crate::有序映射::有序映射;
use crate::语法映射::{加载样式方案, 样式信息};
use crate::调色板::{合成透明颜色, 色阶, 调色板对象};
use crate::错误::结果;

/// 生成工作台颜色表；与 Python 实现一致，过滤空值并保持登记顺序。
///
/// # Errors
/// 方案文件无法读取或解析时返回错误。
pub fn 生成界面颜色(
    调色板: &调色板对象,
    彩色状态栏: bool,
    根目录: &Path,
) -> 结果<有序映射> {
    let dark = 调色板.mode == "dark";
    let 样式表 = 加载样式方案(根目录, &调色板.mode)?.styles;

    // 浅色标题栏与编辑区同为白色，标签条带需独立的中性色阶。
    let 浅色标签 = !dark && !调色板.high_contrast;
    let 标签背景 = if 浅色标签 {
        表面叠加(调色板, &取(调色板, "fg"), 0.06, "bg_window")
    } else {
        取(调色板, "bg_headerbar")
    };
    let 活动标签 = if 浅色标签 {
        取(调色板, "bg_view")
    } else {
        表面叠加(调色板, &取(调色板, "fg"), 0.10, "bg_headerbar")
    };
    let 非活动标签前景 = if 浅色标签 {
        表面叠加(调色板, &取(调色板, "fg"), 0.75, "bg_window")
    } else {
        取(调色板, "fg_dim_headerbar")
    };

    let mut 颜色 = 颜色表::新();
    // --- 全局 ---
    颜色.放("foreground", 取(调色板, "fg"));
    颜色.放("disabledForeground", 取(调色板, "fg_disabled"));
    颜色.放("errorForeground", 取(调色板, "error_standalone"));
    颜色.放("descriptionForeground", 取(调色板, "fg_dim"));
    颜色.放("icon.foreground", 取(调色板, "fg_dim"));
    颜色.放("focusBorder", 透明(&取(调色板, "accent_bg"), 0.75));
    // contrastBorder/contrastActiveBorder 仅属于高对比度主题：VS Code 的 CSS
    // 以 `unset`/`transparent` 作为回退，普通主题里定义它们会到处多出描边。
    颜色.放可选(
        "contrastBorder",
        if 调色板.high_contrast {
            Some(取(调色板, "contrast_border"))
        } else {
            None
        },
    );
    颜色.放可选(
        "contrastActiveBorder",
        if 调色板.high_contrast {
            Some(透明(&取(调色板, "accent_bg"), 0.75))
        } else {
            None
        },
    );
    颜色.放(
        "widget.shadow",
        if dark {
            "#00000066".to_string()
        } else {
            "#00000026".to_string()
        },
    );
    颜色.放("widget.border", "#00000000");
    颜色.放("selection.background", 透明(&取(调色板, "accent_bg"), 0.35));
    颜色.放("sash.hoverBorder", 透明(&取(调色板, "accent_bg"), 0.75));
    颜色.放("badge.background", 取(调色板, "accent_bg"));
    颜色.放("badge.foreground", 取(调色板, "accent_fg"));
    颜色.放("progressBar.background", 取(调色板, "accent_bg"));
    颜色.放("textLink.foreground", 取(调色板, "accent_standalone"));
    颜色.放("textLink.activeForeground", 取(调色板, "accent_hover"));
    颜色.放("textPreformat.foreground", 色调(调色板, "teal", 4, 5));
    颜色.放("textBlockQuote.background", 取(调色板, "bg_hover_view"));
    颜色.放("textBlockQuote.border", 取(调色板, "border"));
    颜色.放("textCodeBlock.background", 取(调色板, "bg_hover_view"));
    颜色.放("textSeparator.foreground", 取(调色板, "border_dim"));
    颜色.放("keybindingLabel.background", 取(调色板, "bg_card"));
    颜色.放("keybindingLabel.foreground", 取(调色板, "fg_card"));
    颜色.放("keybindingLabel.border", 取(调色板, "border"));
    颜色.放("keybindingLabel.bottomBorder", 取(调色板, "border_dim"));
    颜色.放(
        "keybindingTable.headerBackground",
        取(调色板, "bg_headerbar"),
    );
    颜色.放("keybindingTable.rowsBackground", 取(调色板, "bg_window"));
    颜色.放("banner.background", 取(调色板, "warning_bg"));
    颜色.放("banner.foreground", 取(调色板, "warning_fg"));
    颜色.放("banner.iconForeground", 取(调色板, "warning_fg"));
    // --- 窗口 / 标题栏 ---
    颜色.放("window.activeBorder", "#00000000");
    颜色.放("window.inactiveBorder", "#00000000");
    颜色.放("titleBar.activeBackground", 取(调色板, "bg_headerbar"));
    颜色.放("titleBar.activeForeground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "titleBar.inactiveBackground",
        取(调色板, "bg_headerbar_backdrop"),
    );
    颜色.放(
        "titleBar.inactiveForeground",
        取(调色板, "fg_dim_headerbar"),
    );
    颜色.放("titleBar.border", "#00000000");
    // --- 活动栏 ---
    颜色.放("activityBar.background", 取(调色板, "bg_sidebar"));
    颜色.放("activityBar.foreground", 取(调色板, "fg_sidebar"));
    颜色.放(
        "activityBar.inactiveForeground",
        取(调色板, "fg_dim_sidebar"),
    );
    颜色.放(
        "activityBar.border",
        if 调色板.high_contrast {
            取(调色板, "sidebar_border")
        } else {
            "#00000000".to_string()
        },
    );
    颜色.放(
        "activityBar.activeBackground",
        取(调色板, "bg_active_sidebar"),
    );
    颜色.放("activityBar.activeBorder", "#00000000");
    颜色.放(
        "activityBar.activeFocusBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放(
        "activityBar.dropBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放("activityBarTop.background", 取(调色板, "bg_sidebar"));
    颜色.放("activityBarTop.foreground", 取(调色板, "fg_sidebar"));
    颜色.放(
        "activityBarTop.inactiveForeground",
        取(调色板, "fg_dim_sidebar"),
    );
    颜色.放("activityBarTop.activeBorder", "#00000000");
    颜色.放(
        "activityBarTop.activeBackground",
        取(调色板, "bg_active_sidebar"),
    );
    颜色.放(
        "activityBarTop.dropBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放("activityBarBadge.background", 取(调色板, "accent_bg"));
    颜色.放("activityBarBadge.foreground", 取(调色板, "accent_fg"));
    颜色.放("activityErrorBadge.background", 取(调色板, "error_bg"));
    颜色.放("activityErrorBadge.foreground", 取(调色板, "error_fg"));
    颜色.放("activityWarningBadge.background", 取(调色板, "warning_bg"));
    颜色.放("activityWarningBadge.foreground", 取(调色板, "warning_fg"));
    // --- 侧栏 ---
    颜色.放("sideBar.background", 取(调色板, "bg_sidebar"));
    颜色.放("sideBar.foreground", 取(调色板, "fg_sidebar"));
    颜色.放("sideBar.border", 取(调色板, "sidebar_border"));
    颜色.放(
        "sideBar.dropBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("sideBarTitle.foreground", 取(调色板, "fg_sidebar"));
    颜色.放("sideBarSectionHeader.background", "#00000000");
    颜色.放(
        "sideBarSectionHeader.foreground",
        取(调色板, "fg_dim_sidebar"),
    );
    颜色.放("sideBarSectionHeader.border", 取(调色板, "border_dim"));
    颜色.放("sideBarActivityBarTop.border", "#00000000");
    // --- 状态栏 ---
    颜色.放("statusBar.background", 取(调色板, "bg_headerbar"));
    颜色.放("statusBar.foreground", 取(调色板, "fg_headerbar"));
    颜色.放("statusBar.border", "#00000000");
    颜色.放("statusBar.debuggingBackground", 取(调色板, "warning_bg"));
    颜色.放("statusBar.debuggingForeground", 取(调色板, "warning_fg"));
    颜色.放("statusBar.debuggingBorder", "#00000000");
    颜色.放("statusBar.noFolderBackground", 取(调色板, "bg_headerbar"));
    颜色.放("statusBar.noFolderForeground", 取(调色板, "fg_headerbar"));
    颜色.放("statusBar.noFolderBorder", "#00000000");
    颜色.放(
        "statusBarItem.hoverBackground",
        取(调色板, "bg_hover_headerbar"),
    );
    颜色.放("statusBarItem.hoverForeground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "statusBarItem.activeBackground",
        取(调色板, "bg_active_headerbar"),
    );
    颜色.放(
        "statusBarItem.prominentBackground",
        取(调色板, "bg_active_headerbar"),
    );
    颜色.放(
        "statusBarItem.prominentForeground",
        取(调色板, "fg_headerbar"),
    );
    颜色.放(
        "statusBarItem.prominentHoverBackground",
        取(调色板, "bg_active_headerbar"),
    );
    颜色.放("statusBarItem.remoteBackground", 取(调色板, "accent_bg"));
    颜色.放("statusBarItem.remoteForeground", 取(调色板, "accent_fg"));
    颜色.放(
        "statusBarItem.remoteHoverBackground",
        取(调色板, "accent_hover"),
    );
    颜色.放(
        "statusBarItem.remoteHoverForeground",
        取(调色板, "accent_fg"),
    );
    颜色.放("statusBarItem.errorBackground", 取(调色板, "error_bg"));
    颜色.放("statusBarItem.errorForeground", 取(调色板, "error_fg"));
    颜色.放(
        "statusBarItem.errorHoverBackground",
        取(调色板, "destructive_standalone"),
    );
    颜色.放("statusBarItem.errorHoverForeground", 取(调色板, "error_fg"));
    颜色.放("statusBarItem.warningBackground", 取(调色板, "warning_bg"));
    颜色.放("statusBarItem.warningForeground", 取(调色板, "warning_fg"));
    颜色.放(
        "statusBarItem.warningHoverBackground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "statusBarItem.warningHoverForeground",
        if dark {
            取(调色板, "warning_fg")
        } else {
            取(调色板, "accent_fg")
        },
    );
    颜色.放(
        "statusBarItem.offlineBackground",
        取(调色板, "destructive_bg"),
    );
    颜色.放(
        "statusBarItem.offlineForeground",
        取(调色板, "destructive_fg"),
    );
    颜色.放(
        "statusBarItem.offlineHoverBackground",
        取(调色板, "destructive_standalone"),
    );
    颜色.放(
        "statusBarItem.offlineHoverForeground",
        取(调色板, "destructive_fg"),
    );
    // --- 编辑器 ---
    颜色.放("editor.background", 取(调色板, "editor_bg"));
    颜色.放("editor.foreground", 取(调色板, "editor_fg"));
    颜色.放("editorPane.background", 取(调色板, "editor_bg"));
    颜色.放("editorCursor.foreground", 取(调色板, "editor_cursor"));
    颜色.放("editorCursor.background", 取(调色板, "editor_bg"));
    颜色.放(
        "editorLineNumber.foreground",
        取(调色板, "editor_line_number"),
    );
    颜色.放("editorLineNumber.activeForeground", 取(调色板, "editor_fg"));
    颜色.放(
        "editorLineNumber.dimmedForeground",
        取(调色板, "fg_disabled_view"),
    );
    颜色.放(
        "editor.lineHighlightBackground",
        取(调色板, "editor_line_highlight"),
    );
    颜色.放("editor.lineHighlightBorder", "#00000000");
    颜色.放("editor.selectionBackground", 取(调色板, "selection"));
    颜色.放(
        "editor.inactiveSelectionBackground",
        取(调色板, "selection_inactive"),
    );
    颜色.放(
        "editor.selectionHighlightBackground",
        取(调色板, "selection_highlight"),
    );
    颜色.放("editor.selectionHighlightBorder", "#00000000");
    颜色.放(
        "editor.wordHighlightBackground",
        取(调色板, "word_highlight"),
    );
    颜色.放(
        "editor.wordHighlightStrongBackground",
        取(调色板, "word_highlight_strong"),
    );
    颜色.放(
        "editor.wordHighlightTextBackground",
        取(调色板, "word_highlight_strong"),
    );
    颜色.放("editor.findMatchBackground", 取(调色板, "find_match"));
    颜色.放(
        "editor.findMatchForeground",
        取(调色板, "editor_search_match_fg"),
    );
    颜色.放(
        "editor.findMatchHighlightBackground",
        取(调色板, "find_match_highlight"),
    );
    颜色.放(
        "editor.findRangeHighlightBackground",
        取(调色板, "selection_highlight"),
    );
    颜色.放(
        "editor.hoverHighlightBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放(
        "editor.rangeHighlightBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放("editor.rangeHighlightBorder", "#00000000");
    颜色.放("editorWhitespace.foreground", 取(调色板, "editor_ruler"));
    颜色.放("editorIndentGuide.background1", 取(调色板, "indent_guide"));
    颜色.放(
        "editorIndentGuide.activeBackground1",
        取(调色板, "indent_guide_active"),
    );
    颜色.放("editorRuler.foreground", 取(调色板, "editor_ruler"));
    颜色.放("editorBracketMatch.background", 取(调色板, "bracket_match"));
    颜色.放("editorBracketMatch.border", 取(调色板, "bracket_border"));
    颜色.放(
        "editorBracketHighlight.foreground1",
        色调(调色板, "blue", 2, 4),
    );
    颜色.放(
        "editorBracketHighlight.foreground2",
        色调(调色板, "orange", 2, 4),
    );
    颜色.放(
        "editorBracketHighlight.foreground3",
        色调(调色板, "purple", 1, 4),
    );
    颜色.放(
        "editorBracketHighlight.foreground4",
        色调(调色板, "green", 2, 5),
    );
    颜色.放(
        "editorBracketHighlight.foreground5",
        色调(调色板, "pink", 2, 5),
    );
    颜色.放(
        "editorBracketHighlight.foreground6",
        色调(调色板, "teal", 2, 5),
    );
    颜色.放(
        "editorBracketHighlight.unexpectedBracket.foreground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "editorLink.activeForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("editorPlaceholder.foreground", 取(调色板, "fg_placeholder"));
    颜色.放("editorGhostText.foreground", 取(调色板, "fg_disabled_view"));
    颜色.放("editorGhostText.background", "#00000000");
    颜色.放("editorCodeLens.foreground", 取(调色板, "fg_dim_view"));
    颜色.放(
        "editorLightBulb.foreground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "editorLightBulbAutoFix.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorLightBulbAi.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("editorError.foreground", 取(调色板, "error_standalone"));
    颜色.放(
        "editorError.background",
        透明(&取(调色板, "error_bg"), 0.12),
    );
    颜色.放("editorWarning.foreground", 取(调色板, "warning_standalone"));
    颜色.放(
        "editorWarning.background",
        透明(&取(调色板, "warning_bg"), 0.12),
    );
    颜色.放("editorInfo.foreground", 取(调色板, "accent_standalone"));
    颜色.放(
        "editorInfo.background",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("editorHint.foreground", 取(调色板, "success_standalone"));
    颜色.放("editorHint.border", 取(调色板, "success_bg"));
    颜色.放("editorUnnecessaryCode.border", 取(调色板, "border"));
    颜色.放("editorInlayHint.foreground", 取(调色板, "fg_dim_view"));
    颜色.放("editorInlayHint.background", 取(调色板, "bg_hover_view"));
    颜色.放("editorInlayHint.typeForeground", 取(调色板, "fg_dim_view"));
    颜色.放(
        "editorInlayHint.typeBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放(
        "editorInlayHint.parameterForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "editorInlayHint.parameterBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放(
        "editorUnicodeHighlight.border",
        取(调色板, "warning_standalone"),
    );
    颜色.放("editorUnicodeHighlight.background", "#00000000");
    颜色.放(
        "editorCommentsWidget.rangeBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放(
        "editorCommentsWidget.rangeActiveBackground",
        透明(&取(调色板, "accent_bg"), 0.2),
    );
    颜色.放(
        "editorCommentsWidget.replyInputBackground",
        取(调色板, "bg_input"),
    );
    颜色.放(
        "editorCommentsWidget.resolvedBorder",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorCommentsWidget.unresolvedBorder",
        取(调色板, "accent_standalone"),
    );
    // 装订线（gutter）
    颜色.放("editorGutter.background", 取(调色板, "editor_bg"));
    颜色.放(
        "editorGutter.addedBackground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorGutter.modifiedBackground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorGutter.deletedBackground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "editorGutter.foldingControlForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "editorGutter.commentRangeForeground",
        取(调色板, "fg_disabled_view"),
    );
    颜色.放(
        "editorGutter.addedSecondaryBackground",
        取(调色板, "success_bg"),
    );
    颜色.放(
        "editorGutter.modifiedSecondaryBackground",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "editorGutter.deletedSecondaryBackground",
        取(调色板, "error_bg"),
    );
    // 概览标尺
    颜色.放("editorOverviewRuler.border", "#00000000");
    颜色.放(
        "editorOverviewRuler.addedForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.deletedForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.modifiedForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.errorForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.warningForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.infoForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.findMatchForeground",
        取(调色板, "find_match"),
    );
    颜色.放(
        "editorOverviewRuler.bracketMatchForeground",
        取(调色板, "bracket_border"),
    );
    颜色.放(
        "editorOverviewRuler.selectionHighlightForeground",
        取(调色板, "selection_highlight"),
    );
    颜色.放(
        "editorOverviewRuler.wordHighlightForeground",
        取(调色板, "word_highlight"),
    );
    颜色.放(
        "editorOverviewRuler.wordHighlightStrongForeground",
        取(调色板, "word_highlight_strong"),
    );
    // --- 编辑器组 / 标签页 ---
    颜色.放("editorGroup.border", 取(调色板, "border"));
    颜色.放(
        "editorGroup.dropBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("editorGroup.dropIntoPromptForeground", 取(调色板, "fg_dim"));
    颜色.放(
        "editorGroup.dropIntoPromptBackground",
        取(调色板, "bg_popover"),
    );
    颜色.放("editorGroup.emptyBackground", 取(调色板, "bg_window"));
    颜色.放("editorGroup.focusedEmptyBorder", "#00000000");
    颜色.放("editorGroupHeader.tabsBackground", 标签背景.clone());
    颜色.放(
        "editorGroupHeader.tabsBorder",
        if 浅色标签 {
            取(调色板, "border")
        } else {
            "#00000000".to_string()
        },
    );
    颜色.放("editorGroupHeader.border", "#00000000");
    颜色.放(
        "editorGroupHeader.noTabsBackground",
        取(调色板, "bg_window"),
    );
    颜色.放(
        "editorGroupHeader.connectedTabsBackground",
        标签背景.clone(),
    );
    颜色.放("tab.activeBackground", 活动标签.clone());
    颜色.放("tab.activeForeground", 取(调色板, "fg_headerbar"));
    颜色.放("tab.activeBorder", "#00000000");
    颜色.放("tab.activeBorderTop", "#00000000");
    颜色.放("tab.activeModifiedBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "tab.unfocusedActiveBackground",
        if 浅色标签 {
            活动标签.clone()
        } else {
            表面叠加(调色板, &取(调色板, "fg"), 0.06, "bg_headerbar_backdrop")
        },
    );
    颜色.放(
        "tab.unfocusedActiveForeground",
        取(调色板, "fg_dim_headerbar"),
    );
    颜色.放("tab.unfocusedActiveBorder", "#00000000");
    颜色.放("tab.unfocusedActiveBorderTop", "#00000000");
    颜色.放(
        "tab.unfocusedActiveModifiedBorder",
        取(调色板, "border_dim"),
    );
    颜色.放("tab.inactiveBackground", 标签背景.clone());
    颜色.放("tab.inactiveForeground", 非活动标签前景.clone());
    颜色.放("tab.inactiveModifiedBorder", 取(调色板, "border_dim"));
    颜色.放(
        "tab.unfocusedInactiveBackground",
        if 浅色标签 {
            标签背景.clone()
        } else {
            取(调色板, "bg_headerbar_backdrop")
        },
    );
    颜色.放(
        "tab.unfocusedInactiveForeground",
        取(调色板, "fg_disabled_headerbar"),
    );
    颜色.放("tab.hoverBackground", 取(调色板, "bg_hover_headerbar"));
    颜色.放("tab.hoverForeground", 取(调色板, "fg_headerbar"));
    颜色.放("tab.hoverBorder", "#00000000");
    颜色.放(
        "tab.unfocusedHoverBackground",
        取(调色板, "bg_hover_headerbar"),
    );
    颜色.放("tab.unfocusedHoverForeground", 取(调色板, "fg_headerbar"));
    颜色.放("tab.unfocusedHoverBorder", "#00000000");
    颜色.放("tab.border", "#00000000");
    颜色.放("tab.lastPinnedBorder", 取(调色板, "border_dim"));
    颜色.放("tab.selectedBorderTop", "#00000000");
    颜色.放("tab.selectedBackground", 活动标签.clone());
    颜色.放("tab.selectedForeground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "tab.dragAndDropBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    // --- 面包屑 ---
    颜色.放("breadcrumb.background", 取(调色板, "editor_bg"));
    颜色.放("breadcrumb.foreground", 取(调色板, "fg_dim_view"));
    颜色.放("breadcrumb.focusForeground", 取(调色板, "fg_view"));
    颜色.放(
        "breadcrumb.activeSelectionForeground",
        取(调色板, "fg_view"),
    );
    颜色.放("breadcrumbPicker.background", 取(调色板, "bg_popover"));
    // --- 浮层组件 ---
    颜色.放("editorWidget.background", 取(调色板, "bg_popover"));
    颜色.放("editorWidget.foreground", 取(调色板, "fg_popover"));
    颜色.放("editorWidget.border", 取(调色板, "border"));
    颜色.放("editorWidget.resizeBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "editorWidget.shadow",
        if dark {
            "#00000066".to_string()
        } else {
            "#00000026".to_string()
        },
    );
    颜色.放("editorWidget.prominentForeground", 取(调色板, "fg_popover"));
    颜色.放("editorWidget.prominentBorder", 取(调色板, "border"));
    颜色.放(
        "editorWidget.prominentBackground",
        取(调色板, "bg_hover_popover"),
    );
    颜色.放("editorWidget.errorBorder", 取(调色板, "error_bg"));
    颜色.放("editorWidget.warningBorder", 取(调色板, "warning_bg"));
    颜色.放("editorWidget.infoBorder", 取(调色板, "accent_bg"));
    颜色.放("editorHoverWidget.background", 取(调色板, "bg_popover"));
    颜色.放("editorHoverWidget.foreground", 取(调色板, "fg_popover"));
    颜色.放("editorHoverWidget.border", 取(调色板, "border"));
    颜色.放(
        "editorHoverWidget.statusBarBackground",
        取(调色板, "bg_headerbar"),
    );
    颜色.放(
        "editorHoverWidget.highlightForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("editorSuggestWidget.background", 取(调色板, "bg_popover"));
    颜色.放("editorSuggestWidget.border", 取(调色板, "border"));
    颜色.放("editorSuggestWidget.foreground", 取(调色板, "fg_popover"));
    颜色.放(
        "editorSuggestWidget.focusHighlightForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorSuggestWidget.highlightForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorSuggestWidget.selectedBackground",
        表面叠加(调色板, &取(调色板, "accent_bg"), 0.25, "bg_popover"),
    );
    颜色.放(
        "editorSuggestWidget.selectedForeground",
        取(调色板, "fg_popover"),
    );
    颜色.放(
        "editorSuggestWidget.selectedIconForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorSuggestWidgetStatus.foreground",
        取(调色板, "fg_dim_popover"),
    );
    颜色.放(
        "editorMarkerNavigation.background",
        取(调色板, "bg_popover"),
    );
    颜色.放(
        "editorMarkerNavigationError.background",
        取(调色板, "error_bg"),
    );
    颜色.放(
        "editorMarkerNavigationWarning.background",
        取(调色板, "warning_bg"),
    );
    颜色.放(
        "editorMarkerNavigationInfo.background",
        取(调色板, "accent_bg"),
    );
    颜色.放("editorStickyScroll.background", 取(调色板, "bg_card"));
    颜色.放("editorStickyScroll.border", 取(调色板, "border"));
    颜色.放("editorStickyScroll.shadow", "#000000ff");
    颜色.放(
        "editorStickyScrollHover.background",
        取(调色板, "bg_hover_card"),
    );
    颜色.放("inlineChat.background", 取(调色板, "bg_popover"));
    颜色.放("inlineChat.foreground", 取(调色板, "fg_popover"));
    颜色.放("inlineChat.border", 取(调色板, "border"));
    颜色.放(
        "inlineChat.shadow",
        if dark {
            "#00000066".to_string()
        } else {
            "#00000026".to_string()
        },
    );
    颜色.放("inlineChatInput.background", 取(调色板, "bg_input"));
    颜色.放("inlineChatInput.border", 取(调色板, "border_input"));
    颜色.放(
        "inlineChatInput.focusBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放(
        "inlineChatInput.placeholderForeground",
        取(调色板, "fg_placeholder"),
    );
    颜色.放(
        "inlineChatDiff.inserted",
        透明(&取(调色板, "success_bg"), 0.2),
    );
    颜色.放("inlineChatDiff.removed", 透明(&取(调色板, "error_bg"), 0.2));
    // --- 列表 / 菜单 ---
    颜色.放(
        "list.activeSelectionBackground",
        表面叠加(调色板, &取(调色板, "accent_bg"), 0.25, "bg_sidebar"),
    );
    颜色.放("list.activeSelectionForeground", 取(调色板, "fg_sidebar"));
    颜色.放(
        "list.activeSelectionIconForeground",
        取(调色板, "fg_sidebar"),
    );
    颜色.放(
        "list.inactiveSelectionBackground",
        取(调色板, "bg_active_sidebar"),
    );
    颜色.放("list.inactiveSelectionForeground", 取(调色板, "fg_sidebar"));
    颜色.放(
        "list.inactiveSelectionIconForeground",
        取(调色板, "fg_dim_sidebar"),
    );
    颜色.放(
        "list.focusBackground",
        表面叠加(调色板, &取(调色板, "accent_bg"), 0.25, "bg_sidebar"),
    );
    颜色.放("list.focusForeground", 取(调色板, "fg_sidebar"));
    颜色.放("list.focusOutline", 透明(&取(调色板, "accent_bg"), 0.75));
    颜色.放(
        "list.focusAndSelectionOutline",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放("list.hoverBackground", 取(调色板, "bg_hover_sidebar"));
    颜色.放("list.hoverForeground", 取(调色板, "fg_sidebar"));
    颜色.放("list.highlightForeground", 取(调色板, "accent_standalone"));
    颜色.放(
        "list.focusHighlightForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("list.dropBackground", 透明(&取(调色板, "accent_bg"), 0.12));
    颜色.放("list.errorForeground", 取(调色板, "error_standalone"));
    颜色.放("list.warningForeground", 取(调色板, "warning_standalone"));
    颜色.放(
        "list.deemphasizedForeground",
        取(调色板, "fg_disabled_sidebar"),
    );
    颜色.放("listFilterWidget.background", 取(调色板, "bg_popover"));
    颜色.放("listFilterWidget.outline", "#00000000");
    颜色.放("listFilterWidget.noMatchesOutline", 取(调色板, "error_bg"));
    颜色.放(
        "listFilterWidget.shadow",
        if dark {
            "#00000066".to_string()
        } else {
            "#00000026".to_string()
        },
    );
    颜色.放("tree.indentGuidesStroke", 取(调色板, "indent_guide"));
    颜色.放("tree.inactiveIndentGuidesStroke", 取(调色板, "border_dim"));
    颜色.放("tree.tableColumnsBorder", 取(调色板, "border_dim"));
    颜色.放("tree.tableOddRowsBackground", "#00000000");
    颜色.放("menu.background", 取(调色板, "bg_popover"));
    颜色.放("menu.foreground", 取(调色板, "fg_popover"));
    颜色.放("menu.border", 取(调色板, "border"));
    颜色.放(
        "menu.selectionBackground",
        表面叠加(调色板, &取(调色板, "fg_popover"), 0.1, "bg_popover"),
    );
    颜色.放("menu.selectionForeground", 取(调色板, "fg_popover"));
    颜色.放("menu.selectionBorder", "#00000000");
    颜色.放("menu.separatorBackground", 取(调色板, "border_dim"));
    颜色.放(
        "menubar.selectionBackground",
        取(调色板, "bg_hover_popover"),
    );
    颜色.放("menubar.selectionForeground", 取(调色板, "fg_popover"));
    颜色.放("menubar.selectionBorder", "#00000000");
    // --- 输入控件 ---
    颜色.放("input.background", 取(调色板, "bg_input"));
    颜色.放("input.foreground", 取(调色板, "fg_view"));
    颜色.放("input.border", 取(调色板, "border_input"));
    颜色.放("input.placeholderForeground", 取(调色板, "fg_placeholder"));
    颜色.放("inputOption.activeBackground", 取(调色板, "accent_bg"));
    颜色.放("inputOption.activeForeground", 取(调色板, "accent_fg"));
    颜色.放("inputOption.activeBorder", 取(调色板, "accent_bg"));
    颜色.放("inputOption.hoverBackground", 取(调色板, "bg_hover_view"));
    颜色.放(
        "inputValidation.errorBackground",
        表面叠加(调色板, &取(调色板, "error_bg"), 0.15, "bg_input"),
    );
    颜色.放(
        "inputValidation.errorForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放("inputValidation.errorBorder", 取(调色板, "error_bg"));
    颜色.放(
        "inputValidation.warningBackground",
        表面叠加(调色板, &取(调色板, "warning_bg"), 0.15, "bg_input"),
    );
    颜色.放(
        "inputValidation.warningForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放("inputValidation.warningBorder", 取(调色板, "warning_bg"));
    颜色.放(
        "inputValidation.infoBackground",
        表面叠加(调色板, &取(调色板, "accent_bg"), 0.15, "bg_input"),
    );
    颜色.放(
        "inputValidation.infoForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("inputValidation.infoBorder", 取(调色板, "accent_bg"));
    颜色.放("dropdown.background", 取(调色板, "bg_popover"));
    颜色.放("dropdown.foreground", 取(调色板, "fg_popover"));
    颜色.放("dropdown.border", 取(调色板, "border_input"));
    颜色.放("dropdown.listBackground", 取(调色板, "bg_popover"));
    颜色.放("checkbox.background", 取(调色板, "bg_input"));
    颜色.放("checkbox.foreground", 取(调色板, "fg_view"));
    颜色.放("checkbox.border", 取(调色板, "border_input"));
    颜色.放("checkbox.selectBackground", 取(调色板, "bg_active_view"));
    颜色.放(
        "checkbox.selectBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放("radio.activeForeground", 取(调色板, "accent_standalone"));
    颜色.放("radio.inactiveForeground", 取(调色板, "fg_dim_view"));
    颜色.放("radio.activeBackground", "#00000000");
    颜色.放("radio.inactiveBackground", "#00000000");
    颜色.放("radio.activeBorder", 取(调色板, "accent_bg"));
    颜色.放("radio.inactiveBorder", 取(调色板, "border_input"));
    颜色.放("radio.inactiveHoverBackground", "#00000000");
    // --- 按钮 ---
    颜色.放("button.background", 取(调色板, "accent_bg"));
    颜色.放("button.foreground", 取(调色板, "accent_fg"));
    颜色.放("button.border", 取(调色板, "accent_bg"));
    颜色.放("button.hoverBackground", 取(调色板, "accent_hover"));
    颜色.放("button.separator", "#00000000");
    颜色.放("button.secondaryBackground", 取(调色板, "bg_button"));
    颜色.放("button.secondaryForeground", 取(调色板, "fg"));
    颜色.放("button.secondaryBorder", 取(调色板, "bg_button"));
    颜色.放(
        "button.secondaryHoverBackground",
        取(调色板, "bg_button_hover"),
    );
    颜色.放("extensionButton.background", 取(调色板, "accent_bg"));
    颜色.放("extensionButton.foreground", 取(调色板, "accent_fg"));
    颜色.放(
        "extensionButton.hoverBackground",
        取(调色板, "accent_hover"),
    );
    颜色.放("extensionButton.border", 取(调色板, "accent_bg"));
    颜色.放("extensionButton.separator", "#00000000");
    颜色.放(
        "extensionButton.prominentBackground",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "extensionButton.prominentForeground",
        取(调色板, "accent_fg"),
    );
    颜色.放(
        "extensionButton.prominentHoverBackground",
        取(调色板, "accent_hover"),
    );
    颜色.放("extensionButton.prominentBorder", 取(调色板, "accent_bg"));
    颜色.放("extensionBadge.remoteBackground", 取(调色板, "accent_bg"));
    颜色.放("extensionBadge.remoteForeground", 取(调色板, "accent_fg"));
    颜色.放("profileBadge.background", 取(调色板, "accent_bg"));
    颜色.放("profileBadge.foreground", 取(调色板, "accent_fg"));
    // --- 快速输入 ---
    颜色.放("quickInput.background", 取(调色板, "bg_popover"));
    颜色.放("quickInput.foreground", 取(调色板, "fg_popover"));
    颜色.放("quickInputTitle.background", 取(调色板, "bg_headerbar"));
    颜色.放(
        "quickInputList.focusBackground",
        表面叠加(调色板, &取(调色板, "accent_bg"), 0.25, "bg_popover"),
    );
    颜色.放("quickInputList.focusForeground", 取(调色板, "fg_popover"));
    颜色.放(
        "quickInputList.focusIconForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("pickerGroup.foreground", 取(调色板, "fg_dim_popover"));
    颜色.放("pickerGroup.border", 取(调色板, "border_dim"));
    // --- 面板 / 输出 / 终端 ---
    颜色.放("panel.background", 取(调色板, "bg_window"));
    颜色.放(
        "panel.border",
        if 调色板.high_contrast {
            取(调色板, "border")
        } else {
            取(调色板, "border_dim")
        },
    );
    颜色.放("panel.dropBorder", 透明(&取(调色板, "accent_bg"), 0.75));
    颜色.放("panelTitle.activeForeground", 取(调色板, "fg_window"));
    颜色.放("panelTitle.activeBorder", 取(调色板, "accent_bg"));
    颜色.放("panelTitle.inactiveForeground", 取(调色板, "fg_dim_window"));
    颜色.放("panelTitle.border", "#00000000");
    颜色.放("panelSection.border", 取(调色板, "border_dim"));
    颜色.放(
        "panelSection.dropBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("panelSectionHeader.background", 取(调色板, "bg_window"));
    颜色.放("panelSectionHeader.foreground", 取(调色板, "fg_dim_window"));
    颜色.放("panelSectionHeader.border", 取(调色板, "border_dim"));
    颜色.放("panelStickyScroll.background", 取(调色板, "bg_card"));
    颜色.放("panelStickyScroll.border", 取(调色板, "border"));
    颜色.放("terminal.background", 取(调色板, "bg_view"));
    颜色.放("terminal.foreground", 取(调色板, "editor_fg"));
    颜色.放("terminal.border", 取(调色板, "border_dim"));
    颜色.放("terminal.selectionBackground", 取(调色板, "selection"));
    颜色.放(
        "terminal.inactiveSelectionBackground",
        取(调色板, "selection_inactive"),
    );
    颜色.放("terminal.selectionForeground", 取(调色板, "editor_fg"));
    颜色.放("terminalCursor.foreground", 取(调色板, "accent_standalone"));
    颜色.放("terminalCursor.background", 取(调色板, "bg_view"));
    颜色.放("terminal.findMatchBackground", 取(调色板, "find_match"));
    颜色.放(
        "terminal.findMatchHighlightBackground",
        取(调色板, "find_match_highlight"),
    );
    颜色.放(
        "terminal.hoverHighlightBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放(
        "terminal.dropBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("terminal.tab.activeBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "terminalCommandDecoration.defaultBackground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "terminalCommandDecoration.successBackground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "terminalCommandDecoration.errorBackground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "ports.iconRunningProcessForeground",
        取(调色板, "success_standalone"),
    );

    // 终端 16 色（明暗模式相同）
    for (键, 值) in crate::调色板::终端颜色() {
        颜色.放(键, 值);
    }
    // --- 问题 / 调试 ---
    颜色.放(
        "problemsErrorIcon.foreground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "problemsWarningIcon.foreground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "problemsInfoIcon.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("debugToolBar.background", 取(调色板, "bg_headerbar"));
    颜色.放("debugToolBar.border", "#00000000");
    颜色.放(
        "debugConsole.errorForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "debugConsole.infoForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "debugConsole.warningForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放("debugConsole.sourceForeground", 取(调色板, "fg_dim"));
    颜色.放(
        "debugConsoleInputIcon.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("debugExceptionWidget.background", 取(调色板, "bg_popover"));
    颜色.放("debugExceptionWidget.border", 取(调色板, "error_bg"));
    颜色.放(
        "debugIcon.breakpointForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "debugIcon.breakpointDisabledForeground",
        取(调色板, "fg_disabled"),
    );
    颜色.放(
        "debugIcon.breakpointUnverifiedForeground",
        取(调色板, "fg_dim"),
    );
    颜色.放(
        "debugIcon.breakpointCurrentStackframeForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "debugIcon.breakpointStackframeForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "debugIcon.startForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放("debugIcon.pauseForeground", 取(调色板, "accent_standalone"));
    颜色.放("debugIcon.stopForeground", 取(调色板, "error_standalone"));
    颜色.放(
        "debugIcon.disconnectForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "debugIcon.restartForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "debugIcon.stepOverForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "debugIcon.stepIntoForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "debugIcon.stepOutForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "debugIcon.continueForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "debugTokenExpression.name",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放(
        "debugTokenExpression.value",
        取样式(&样式表, "def:string", true, &取(调色板, "fg")),
    );
    颜色.放(
        "debugTokenExpression.string",
        取样式(&样式表, "def:string", true, &取(调色板, "fg")),
    );
    颜色.放(
        "debugTokenExpression.boolean",
        取样式(&样式表, "def:boolean", true, &取(调色板, "fg")),
    );
    颜色.放(
        "debugTokenExpression.number",
        取样式(&样式表, "def:number", true, &取(调色板, "fg")),
    );
    颜色.放("debugTokenExpression.error", 取(调色板, "error_standalone"));
    颜色.放("debugView.exceptionLabelBackground", 取(调色板, "error_bg"));
    颜色.放("debugView.exceptionLabelForeground", 取(调色板, "error_fg"));
    颜色.放("debugView.stateLabelBackground", 取(调色板, "accent_bg"));
    颜色.放("debugView.stateLabelForeground", 取(调色板, "accent_fg"));
    颜色.放("debugView.valueChangedHighlight", 取(调色板, "accent_bg"));
    // --- Git / 源代码管理 ---
    颜色.放(
        "gitDecoration.addedResourceForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "gitDecoration.untrackedResourceForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "gitDecoration.renamedResourceForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "gitDecoration.modifiedResourceForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "gitDecoration.stageModifiedResourceForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "gitDecoration.deletedResourceForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "gitDecoration.stageDeletedResourceForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "gitDecoration.conflictingResourceForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "gitDecoration.submoduleResourceForeground",
        色调(调色板, "purple", 1, 4),
    );
    颜色.放(
        "gitDecoration.ignoredResourceForeground",
        取(调色板, "fg_disabled_sidebar"),
    );
    颜色.放("scm.providerBorder", 取(调色板, "border_dim"));
    颜色.放(
        "git.blame.editorDecorationForeground",
        取(调色板, "fg_dim_view"),
    );
    // --- 差异 / 合并 ---
    颜色.放(
        "diffEditor.insertedTextBackground",
        取(调色板, "diff_inserted"),
    );
    颜色.放(
        "diffEditor.removedTextBackground",
        取(调色板, "diff_removed"),
    );
    颜色.放(
        "diffEditor.insertedLineBackground",
        取(调色板, "diff_inserted_bg"),
    );
    颜色.放(
        "diffEditor.removedLineBackground",
        取(调色板, "diff_removed_bg"),
    );
    颜色.放("diffEditor.diagonalFill", 取(调色板, "bg_hover_view"));
    颜色.放(
        "diffEditor.unchangedRegionBackground",
        取(调色板, "bg_window"),
    );
    颜色.放(
        "diffEditor.unchangedRegionForeground",
        取(调色板, "fg_dim_window"),
    );
    颜色.放("diffEditor.unchangedRegionShadow", "#00000000");
    颜色.放("diffEditor.unchangedCodeBackground", "#00000000");
    颜色.放("diffEditor.border", 取(调色板, "border"));
    颜色.放(
        "diffEditorGutter.insertedLineBackground",
        取(调色板, "diff_inserted_bg"),
    );
    颜色.放(
        "diffEditorGutter.removedLineBackground",
        取(调色板, "diff_removed_bg"),
    );
    颜色.放(
        "diffEditorOverview.insertedForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "diffEditorOverview.removedForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放("multiDiffEditor.background", 取(调色板, "editor_bg"));
    颜色.放("multiDiffEditor.border", 取(调色板, "border"));
    颜色.放(
        "multiDiffEditor.headerBackground",
        取(调色板, "bg_headerbar"),
    );
    颜色.放(
        "mergeEditor.change.background",
        透明(&取(调色板, "success_bg"), 0.25),
    );
    颜色.放(
        "mergeEditor.change.word.background",
        透明(&取(调色板, "success_bg"), 0.4),
    );
    颜色.放(
        "mergeEditor.changeBase.background",
        透明(&取(调色板, "fg_dim"), 0.25),
    );
    颜色.放(
        "mergeEditor.changeBase.word.background",
        透明(&取(调色板, "fg_dim"), 0.4),
    );
    颜色.放(
        "mergeEditor.conflict.unhandledFocused.border",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "mergeEditor.conflict.unhandledUnfocused.border",
        取(调色板, "border"),
    );
    颜色.放(
        "mergeEditor.conflict.handledFocused.border",
        取(调色板, "border"),
    );
    颜色.放(
        "mergeEditor.conflict.handledUnfocused.border",
        取(调色板, "border_dim"),
    );
    颜色.放(
        "mergeEditor.conflict.handled.minimapOverViewRuler",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "mergeEditor.conflict.unhandled.minimapOverViewRuler",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "mergeEditor.conflict.input1.background",
        透明(&取(调色板, "accent_bg"), 0.2),
    );
    颜色.放(
        "mergeEditor.conflict.input2.background",
        透明(&取(调色板, "accent_bg"), 0.2),
    );
    // --- Notebook ---
    颜色.放("notebook.editorBackground", 取(调色板, "editor_bg"));
    颜色.放("notebook.cellBorderColor", 取(调色板, "border"));
    颜色.放("notebook.cellHoverBackground", 取(调色板, "bg_hover_view"));
    颜色.放("notebook.cellInsertionIndicator", 取(调色板, "accent_bg"));
    颜色.放(
        "notebook.cellStatusBarItemHoverBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放("notebook.cellToolbarSeparator", 取(调色板, "border_dim"));
    颜色.放(
        "notebook.selectedCellBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("notebook.selectedCellBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "notebook.inactiveSelectedCellBorder",
        取(调色板, "border_input"),
    );
    颜色.放("notebook.focusedCellBorder", 取(调色板, "accent_bg"));
    颜色.放("notebook.focusedEditorBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "notebook.outputContainerBackgroundColor",
        取(调色板, "bg_window"),
    );
    颜色.放("notebook.outputContainerBorderColor", "#00000000");
    // --- 测试 ---
    颜色.放("testing.iconFailed", 取(调色板, "error_standalone"));
    颜色.放("testing.iconErrored", 取(调色板, "error_standalone"));
    颜色.放("testing.iconPassed", 取(调色板, "success_standalone"));
    颜色.放("testing.iconQueued", 取(调色板, "fg_dim"));
    颜色.放("testing.iconUnset", 取(调色板, "fg_disabled"));
    颜色.放("testing.iconSkipped", 取(调色板, "fg_disabled"));
    颜色.放("testing.runAction", 取(调色板, "success_standalone"));
    颜色.放("testing.peekBorder", 取(调色板, "border"));
    颜色.放("testing.peekHeaderBackground", 取(调色板, "bg_headerbar"));
    颜色.放(
        "testing.message.error.lineBackground",
        透明(&取(调色板, "error_bg"), 0.12),
    );
    颜色.放(
        "testing.message.info.decorationForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "testing.message.info.lineBackground",
        透明(&取(调色板, "accent_bg"), 0.1),
    );
    颜色.放("testing.coverCountBadgeBackground", 取(调色板, "accent_bg"));
    颜色.放("testing.coverCountBadgeForeground", 取(调色板, "accent_fg"));
    颜色.放(
        "testing.coveredBackground",
        透明(&取(调色板, "success_bg"), 0.25),
    );
    颜色.放("testing.coveredBorder", 取(调色板, "success_bg"));
    颜色.放(
        "testing.uncoveredBackground",
        透明(&取(调色板, "error_bg"), 0.25),
    );
    颜色.放("testing.uncoveredBorder", 取(调色板, "error_bg"));
    // --- 聊天 / 智能体 ---
    颜色.放("chat.requestBubbleBackground", 取(调色板, "bg_card"));
    颜色.放(
        "chat.requestBubbleHoverBackground",
        取(调色板, "bg_hover_card"),
    );
    颜色.放("chat.avatarBackground", 取(调色板, "accent_bg"));
    颜色.放("chat.avatarForeground", 取(调色板, "accent_fg"));
    颜色.放(
        "chat.slashCommandBackground",
        透明(&取(调色板, "accent_bg"), 0.15),
    );
    颜色.放(
        "chat.slashCommandForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "chat.editedFileForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "chat.linesAddedForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "chat.linesRemovedForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放("chat.inputWorkingBorderColor1", 取(调色板, "accent_bg"));
    颜色.放("chat.thinkingShimmer", 取(调色板, "fg_dim"));
    颜色.放("chat.checkpointSeparator", 取(调色板, "border_dim"));
    颜色.放("agents.background", 取(调色板, "bg_window"));
    颜色.放("agentsPanel.background", 取(调色板, "bg_sidebar"));
    颜色.放("agentsPanel.foreground", 取(调色板, "fg_sidebar"));
    颜色.放("agentsPanel.border", 取(调色板, "border"));
    颜色.放("agentsCard.border", 取(调色板, "border"));
    颜色.放("agentsBottomPanel.border", 取(调色板, "border"));
    颜色.放("agentsChatInput.background", 取(调色板, "bg_input"));
    颜色.放("agentsChatInput.foreground", 取(调色板, "fg_view"));
    颜色.放("agentsChatInput.border", 取(调色板, "border_input"));
    颜色.放(
        "agentsChatInput.focusBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放(
        "agentsChatInput.placeholderForeground",
        取(调色板, "fg_placeholder"),
    );
    颜色.放("agentsNewSessionButton.background", "#00000000");
    颜色.放("agentsNewSessionButton.foreground", 取(调色板, "fg"));
    颜色.放("agentsNewSessionButton.border", 取(调色板, "border_input"));
    颜色.放(
        "agentsNewSessionButton.hoverBackground",
        取(调色板, "bg_hover"),
    );
    颜色.放("agentsBadge.background", 取(调色板, "accent_bg"));
    颜色.放("agentsBadge.foreground", 取(调色板, "accent_fg"));
    颜色.放("agentsUnreadBadge.background", 取(调色板, "accent_bg"));
    颜色.放("agentsUnreadBadge.foreground", 取(调色板, "accent_fg"));
    颜色.放("agentStatusIndicator.background", 取(调色板, "accent_bg"));
    颜色.放("agentsGradient.tintColor", 取(调色板, "accent_bg"));
    // --- 通知 ---
    颜色.放("notifications.background", 取(调色板, "bg_popover"));
    颜色.放("notifications.foreground", 取(调色板, "fg_popover"));
    颜色.放("notifications.border", 取(调色板, "border"));
    颜色.放(
        "notificationsErrorIcon.foreground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "notificationsWarningIcon.foreground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "notificationsInfoIcon.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("notificationCenter.border", 取(调色板, "border"));
    颜色.放(
        "notificationCenterHeader.background",
        取(调色板, "bg_headerbar"),
    );
    颜色.放(
        "notificationCenterHeader.foreground",
        取(调色板, "fg_headerbar"),
    );
    颜色.放(
        "notificationLink.foreground",
        取(调色板, "accent_standalone"),
    );
    // --- 预览 / 评论 / 其他 ---
    颜色.放("peekView.border", 取(调色板, "accent_bg"));
    颜色.放("peekViewEditor.background", 取(调色板, "bg_card"));
    颜色.放("peekViewEditorGutter.background", 取(调色板, "bg_card"));
    颜色.放(
        "peekViewEditor.matchHighlightBackground",
        取(调色板, "find_match"),
    );
    颜色.放("peekViewEditor.matchHighlightBorder", "#00000000");
    颜色.放("peekViewResult.background", 取(调色板, "bg_popover"));
    颜色.放("peekViewResult.fileForeground", 取(调色板, "fg_popover"));
    颜色.放(
        "peekViewResult.lineForeground",
        取(调色板, "fg_dim_popover"),
    );
    颜色.放(
        "peekViewResult.matchHighlightBackground",
        透明(&取(调色板, "accent_bg"), 0.25),
    );
    颜色.放(
        "peekViewResult.selectionBackground",
        透明(&取(调色板, "accent_bg"), 0.3),
    );
    颜色.放(
        "peekViewResult.selectionForeground",
        取(调色板, "fg_popover"),
    );
    颜色.放("peekViewTitle.background", 取(调色板, "bg_headerbar"));
    颜色.放(
        "peekViewTitleDescription.foreground",
        取(调色板, "fg_dim_headerbar"),
    );
    颜色.放("peekViewTitleLabel.foreground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "editor.snippetTabstopHighlightBackground",
        透明(&取(调色板, "accent_bg"), 0.2),
    );
    颜色.放(
        "editor.snippetTabstopHighlightBorder",
        取(调色板, "border_input"),
    );
    颜色.放(
        "editor.snippetFinalTabstopHighlightBackground",
        透明(&取(调色板, "success_bg"), 0.2),
    );
    颜色.放(
        "editor.snippetFinalTabstopHighlightBorder",
        取(调色板, "success_bg"),
    );
    颜色.放(
        "symbolIcon.arrayForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.booleanForeground",
        取样式(&样式表, "def:boolean", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.classForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放("symbolIcon.colorForeground", 色调(调色板, "red", 2, 4));
    颜色.放(
        "symbolIcon.constantForeground",
        取样式(&样式表, "def:constant", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.constructorForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.enumeratorForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.enumeratorMemberForeground",
        取样式(&样式表, "def:constant", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.eventForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.fieldForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放("symbolIcon.fileForeground", 取(调色板, "fg_dim"));
    颜色.放("symbolIcon.folderForeground", 取(调色板, "fg_dim"));
    颜色.放(
        "symbolIcon.functionForeground",
        取样式(&样式表, "def:statement", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.interfaceForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.keyForeground",
        取样式(&样式表, "def:constant", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.keywordForeground",
        取样式(&样式表, "def:statement", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.methodForeground",
        取样式(&样式表, "def:statement", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.moduleForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.namespaceForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.nullForeground",
        取样式(&样式表, "def:constant", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.numberForeground",
        取样式(&样式表, "def:number", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.objectForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.operatorForeground",
        取样式(&样式表, "def:statement", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.packageForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.propertyForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.referenceForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.snippetForeground",
        取样式(&样式表, "def:string", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.stringForeground",
        取样式(&样式表, "def:string", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.structForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放("symbolIcon.textForeground", 取(调色板, "fg_dim"));
    颜色.放(
        "symbolIcon.typeParameterForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.unitForeground",
        取样式(&样式表, "def:number", true, &取(调色板, "fg")),
    );
    颜色.放(
        "symbolIcon.variableForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放("charts.foreground", 取(调色板, "fg"));
    颜色.放("charts.lines", 取(调色板, "border"));
    颜色.放("charts.red", 色调(调色板, "red", 1, 4));
    颜色.放("charts.blue", 色调(调色板, "blue", 1, 4));
    颜色.放("charts.yellow", 色调(调色板, "yellow", 4, 5));
    颜色.放("charts.orange", 色调(调色板, "orange", 1, 4));
    颜色.放("charts.green", 色调(调色板, "green", 1, 5));
    颜色.放("charts.purple", 色调(调色板, "purple", 1, 4));
    // --- 欢迎页 / 引导页 ---
    颜色.放("welcomePage.background", 取(调色板, "bg_window"));
    颜色.放("welcomePage.progress.background", 取(调色板, "accent_bg"));
    颜色.放("welcomePage.progress.foreground", 取(调色板, "accent_fg"));
    颜色.放("welcomePage.tileBackground", 取(调色板, "bg_card"));
    颜色.放(
        "welcomePage.tileHoverBackground",
        取(调色板, "bg_hover_card"),
    );
    颜色.放("welcomePage.tileBorder", 取(调色板, "border"));
    颜色.放(
        "walkThrough.embeddedEditorBackground",
        取(调色板, "bg_window"),
    );
    // --- 补齐其余内置主题键 ---
    颜色.放("actionBar.toggledBackground", 取(调色板, "bg_active_view"));
    颜色.放("browser.border", "#00000000");
    颜色.放(
        "commandCenter.background",
        取(调色板, "bg_button_headerbar"),
    );
    颜色.放("commandCenter.border", "#00000000");
    颜色.放("commandCenter.foreground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "commandCenter.activeBackground",
        取(调色板, "bg_active_headerbar"),
    );
    颜色.放("commandCenter.activeBorder", 取(调色板, "border_input"));
    颜色.放("commandCenter.activeForeground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "editorSuggestWidget.focusOutline",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放("list.invalidItemForeground", 取(调色板, "error_standalone"));
    颜色.放("minimap.background", 取(调色板, "editor_bg"));
    颜色.放("minimap.selectionHighlight", 取(调色板, "selection"));
    颜色.放(
        "minimap.selectionOccurrenceHighlight",
        取(调色板, "word_highlight"),
    );
    颜色.放("minimap.findMatchHighlight", 取(调色板, "find_match"));
    颜色.放("minimap.errorHighlight", 取(调色板, "error_standalone"));
    颜色.放("minimap.warningHighlight", 取(调色板, "warning_standalone"));
    颜色.放(
        "minimapGutter.addedBackground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "minimapGutter.modifiedBackground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "minimapGutter.deletedBackground",
        取(调色板, "error_standalone"),
    );
    颜色.放("minimapSlider.background", 取(调色板, "scrollbar"));
    颜色.放(
        "minimapSlider.hoverBackground",
        取(调色板, "scrollbar_hover"),
    );
    颜色.放(
        "minimapSlider.activeBackground",
        取(调色板, "scrollbar_active"),
    );
    颜色.放("modernActivityBar.border", 取(调色板, "sidebar_border"));
    颜色.放(
        "modernActivityBarItem.activeBackground",
        取(调色板, "bg_active_sidebar"),
    );
    颜色.放(
        "modernActivityBarItem.activeForeground",
        取(调色板, "fg_sidebar"),
    );
    颜色.放(
        "modernActivityBarItem.hoverBackground",
        取(调色板, "bg_hover_sidebar"),
    );
    颜色.放(
        "modernActivityBarItem.hoverForeground",
        取(调色板, "fg_sidebar"),
    );
    颜色.放("notificationToast.border", 取(调色板, "border"));
    颜色.放("panelInput.border", 取(调色板, "border_input"));
    颜色.放("panelStickyScroll.shadow", "#00000000");
    颜色.放(
        "quickInputList.focusHighlightForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("scrollbar.shadow", "#00000000");
    颜色.放("scrollbarSlider.background", 取(调色板, "scrollbar"));
    颜色.放(
        "scrollbarSlider.hoverBackground",
        取(调色板, "scrollbar_hover"),
    );
    颜色.放(
        "scrollbarSlider.activeBackground",
        取(调色板, "scrollbar_active"),
    );
    颜色.放("searchEditor.textInputBorder", 取(调色板, "border_input"));
    颜色.放("settings.dropdownBackground", 取(调色板, "bg_popover"));
    颜色.放("settings.dropdownBorder", 取(调色板, "border_input"));
    颜色.放("settings.headerForeground", 取(调色板, "fg_dim"));
    颜色.放("settings.modifiedItemIndicator", 取(调色板, "accent_bg"));
    颜色.放("settings.numberInputBorder", 取(调色板, "border_input"));
    颜色.放("settings.textInputBorder", 取(调色板, "border_input"));
    颜色.放("sideBarStickyScroll.shadow", "#00000000");
    颜色.放("statusBar.focusBorder", "#00000000");
    颜色.放(
        "statusBar.inactiveBackground",
        取(调色板, "bg_headerbar_backdrop"),
    );
    颜色.放(
        "statusBarItem.compactHoverBackground",
        取(调色板, "bg_hover_headerbar"),
    );
    颜色.放(
        "statusBarItem.focusBorder",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放(
        "statusBarItem.prominentHoverForeground",
        取(调色板, "fg_headerbar"),
    );
    颜色.放("surface.background", 取(调色板, "bg_window"));
    颜色.放("surface.border", 取(调色板, "border"));
    颜色.放("surface.foreground", 取(调色板, "fg"));
    颜色.放("textPreformat.background", 取(调色板, "bg_hover_view"));
    颜色.放("toolbar.activeBackground", 取(调色板, "bg_active"));
    颜色.放("toolbar.hoverBackground", 取(调色板, "bg_hover"));
    // --- 行内编辑 ---
    颜色.放(
        "inlineEdit.originalBackground",
        取(调色板, "diff_removed_bg"),
    );
    颜色.放("inlineEdit.originalBorder", 取(调色板, "error_bg"));
    颜色.放(
        "inlineEdit.originalChangedLineBackground",
        透明(&取(调色板, "error_bg"), 0.25),
    );
    颜色.放(
        "inlineEdit.originalChangedTextBackground",
        透明(&取(调色板, "error_bg"), 0.4),
    );
    颜色.放(
        "inlineEdit.modifiedBackground",
        取(调色板, "diff_inserted_bg"),
    );
    颜色.放("inlineEdit.modifiedBorder", 取(调色板, "success_bg"));
    颜色.放(
        "inlineEdit.modifiedChangedLineBackground",
        透明(&取(调色板, "success_bg"), 0.25),
    );
    颜色.放(
        "inlineEdit.modifiedChangedTextBackground",
        透明(&取(调色板, "success_bg"), 0.4),
    );
    颜色.放(
        "inlineEdit.tabWillAcceptOriginalBorder",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "inlineEdit.tabWillAcceptModifiedBorder",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.background",
        取(调色板, "bg_hover_view"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.primaryBackground",
        透明(&取(调色板, "accent_bg"), 0.25),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.primaryBorder",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.primaryForeground",
        取(调色板, "accent_fg"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.secondaryBackground",
        取(调色板, "bg_hover_view"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.secondaryBorder",
        取(调色板, "border"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.secondaryForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.successfulBackground",
        透明(&取(调色板, "success_bg"), 0.25),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.successfulBorder",
        取(调色板, "success_bg"),
    );
    颜色.放(
        "inlineEdit.gutterIndicator.successfulForeground",
        取(调色板, "success_fg"),
    );
    // --- 终端符号图标 ---
    颜色.放(
        "terminalSymbolIcon.aliasForeground",
        取样式(&样式表, "def:statement", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.argumentForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.branchForeground",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.commitForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "terminalSymbolIcon.fileForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "terminalSymbolIcon.flagForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.folderForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "terminalSymbolIcon.inlineSuggestionForeground",
        取(调色板, "fg_disabled_view"),
    );
    颜色.放(
        "terminalSymbolIcon.methodForeground",
        取样式(&样式表, "def:statement", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.optionForeground",
        取样式(&样式表, "def:identifier", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.optionValueForeground",
        取样式(&样式表, "def:constant", true, &取(调色板, "fg")),
    );
    颜色.放(
        "terminalSymbolIcon.pullRequestForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "terminalSymbolIcon.pullRequestDoneForeground",
        色调(调色板, "purple", 1, 4),
    );
    颜色.放(
        "terminalSymbolIcon.remoteForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "terminalSymbolIcon.stashForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放("terminalSymbolIcon.symbolText", 取(调色板, "fg_dim_view"));
    颜色.放(
        "terminalSymbolIcon.symbolicLinkFileForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "terminalSymbolIcon.symbolicLinkFolderForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "terminalSymbolIcon.tagForeground",
        取样式(&样式表, "def:constant", true, &取(调色板, "fg")),
    );
    // --- 仪表 ---
    颜色.放("gauge.background", 取(调色板, "bg_hover_view"));
    颜色.放("gauge.foreground", 取(调色板, "accent_bg"));
    颜色.放("gauge.border", 取(调色板, "border_input"));
    颜色.放("gauge.errorBackground", 透明(&取(调色板, "error_bg"), 0.25));
    颜色.放("gauge.errorForeground", 取(调色板, "error_bg"));
    颜色.放(
        "gauge.warningBackground",
        透明(&取(调色板, "warning_bg"), 0.25),
    );
    颜色.放("gauge.warningForeground", 取(调色板, "warning_bg"));
    // --- 源代码管理图表 ---
    颜色.放("scmGraph.foreground1", 色调(调色板, "blue", 2, 4));
    颜色.放("scmGraph.foreground2", 色调(调色板, "orange", 2, 4));
    颜色.放("scmGraph.foreground3", 色调(调色板, "green", 2, 5));
    颜色.放("scmGraph.foreground4", 色调(调色板, "purple", 1, 4));
    颜色.放("scmGraph.foreground5", 色调(调色板, "teal", 2, 5));
    颜色.放(
        "scmGraph.historyItemRefColor",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "scmGraph.historyItemRemoteRefColor",
        色调(调色板, "purple", 1, 4),
    );
    颜色.放(
        "scmGraph.historyItemBaseRefColor",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "scmGraph.historyItemHoverDefaultLabelBackground",
        取(调色板, "bg_headerbar"),
    );
    颜色.放(
        "scmGraph.historyItemHoverDefaultLabelForeground",
        取(调色板, "fg_headerbar"),
    );
    颜色.放(
        "scmGraph.historyItemHoverLabelForeground",
        取(调色板, "fg_popover"),
    );
    颜色.放(
        "scmGraph.historyItemHoverAdditionsForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "scmGraph.historyItemHoverDeletionsForeground",
        取(调色板, "error_standalone"),
    );
    // --- 现代 UI ---
    颜色.放("modernUI.shellBackground", 取(调色板, "bg_window"));
    颜色.放(
        "modernUI.inactiveShellBackground",
        取(调色板, "bg_headerbar_backdrop"),
    );
    颜色.放(
        "modernTab.activeBackground",
        表面叠加(调色板, &取(调色板, "fg"), 0.1, "bg_headerbar"),
    );
    颜色.放("modernTab.activeForeground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "modernTab.hoverBackground",
        取(调色板, "bg_hover_headerbar"),
    );
    颜色.放("modernTab.hoverForeground", 取(调色板, "fg_headerbar"));
    颜色.放(
        "modernEditorTab.activeBackground",
        表面叠加(调色板, &取(调色板, "fg"), 0.1, "bg_headerbar"),
    );
    颜色.放(
        "modernEditorTab.activeForeground",
        取(调色板, "fg_headerbar"),
    );
    颜色.放(
        "modernEditorTab.activeHoverBackground",
        表面叠加(调色板, &取(调色板, "fg"), 0.13, "bg_headerbar"),
    );
    颜色.放(
        "modernEditorTab.inactiveBackground",
        取(调色板, "bg_headerbar"),
    );
    颜色.放(
        "modernEditorTab.hoverBackground",
        取(调色板, "bg_hover_headerbar"),
    );
    颜色.放(
        "modernEditorTab.hoverForeground",
        取(调色板, "fg_headerbar"),
    );
    颜色.放(
        "modernEditorTab.activeActionBackground",
        取(调色板, "bg_active_headerbar"),
    );
    颜色.放(
        "modernEditorTab.activeHoverActionBackground",
        表面叠加(调色板, &取(调色板, "fg"), 0.16, "bg_headerbar"),
    );
    颜色.放(
        "modernEditorTab.hoverActionBackground",
        取(调色板, "bg_hover_headerbar"),
    );
    颜色.放(
        "modernEditorTab.selectedActionBackground",
        取(调色板, "bg_active_headerbar"),
    );
    // --- 括号对参考线 ---
    颜色.放(
        "editorBracketPairGuide.background1",
        透明(&色调(调色板, "blue", 2, 4), 0.35),
    );
    颜色.放(
        "editorBracketPairGuide.background2",
        透明(&色调(调色板, "orange", 2, 4), 0.35),
    );
    颜色.放(
        "editorBracketPairGuide.background3",
        透明(&色调(调色板, "purple", 1, 4), 0.35),
    );
    颜色.放(
        "editorBracketPairGuide.background4",
        透明(&色调(调色板, "green", 2, 5), 0.35),
    );
    颜色.放(
        "editorBracketPairGuide.background5",
        透明(&色调(调色板, "pink", 2, 5), 0.35),
    );
    颜色.放(
        "editorBracketPairGuide.background6",
        透明(&色调(调色板, "teal", 2, 5), 0.35),
    );
    颜色.放(
        "editorBracketPairGuide.activeBackground1",
        色调(调色板, "blue", 2, 4),
    );
    颜色.放(
        "editorBracketPairGuide.activeBackground2",
        色调(调色板, "orange", 2, 4),
    );
    颜色.放(
        "editorBracketPairGuide.activeBackground3",
        色调(调色板, "purple", 1, 4),
    );
    颜色.放(
        "editorBracketPairGuide.activeBackground4",
        色调(调色板, "green", 2, 5),
    );
    颜色.放(
        "editorBracketPairGuide.activeBackground5",
        色调(调色板, "pink", 2, 5),
    );
    颜色.放(
        "editorBracketPairGuide.activeBackground6",
        色调(调色板, "teal", 2, 5),
    );
    // --- 概览标尺 ---
    颜色.放("editorOverviewRuler.background", "#00000000");
    颜色.放(
        "editorOverviewRuler.commentForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.commentUnresolvedForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.commentDraftForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.commonContentForeground",
        取(调色板, "fg_dim"),
    );
    颜色.放(
        "editorOverviewRuler.currentContentForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.incomingContentForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.inlineChatInserted",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.inlineChatRemoved",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "editorOverviewRuler.rangeHighlightForeground",
        取(调色板, "find_match"),
    );
    颜色.放(
        "editorOverviewRuler.wordHighlightTextForeground",
        取(调色板, "word_highlight"),
    );
    // --- 设置编辑器 ---
    颜色.放("settings.checkboxBackground", 取(调色板, "bg_input"));
    颜色.放("settings.checkboxBorder", 取(调色板, "border_input"));
    颜色.放("settings.checkboxForeground", 取(调色板, "fg_view"));
    颜色.放("settings.dropdownForeground", 取(调色板, "fg_popover"));
    颜色.放("settings.dropdownListBorder", 取(调色板, "border"));
    颜色.放(
        "settings.focusedRowBackground",
        透明(&取(调色板, "accent_bg"), 0.12),
    );
    颜色.放("settings.focusedRowBorder", 取(调色板, "accent_bg"));
    颜色.放("settings.headerBorder", 取(调色板, "border_dim"));
    颜色.放("settings.sashBorder", 取(调色板, "border_dim"));
    颜色.放("settings.numberInputBackground", 取(调色板, "bg_input"));
    颜色.放("settings.numberInputForeground", 取(调色板, "fg_view"));
    颜色.放("settings.textInputBackground", 取(调色板, "bg_input"));
    颜色.放("settings.textInputForeground", 取(调色板, "fg_view"));
    颜色.放("settings.rowHoverBackground", 取(调色板, "bg_hover"));
    颜色.放("settings.settingsHeaderHoverForeground", 取(调色板, "fg"));
    // --- 测试 ---
    颜色.放(
        "testing.coveredGutterBackground",
        透明(&取(调色板, "success_bg"), 0.4),
    );
    颜色.放(
        "testing.uncoveredGutterBackground",
        透明(&取(调色板, "error_bg"), 0.4),
    );
    颜色.放(
        "testing.uncoveredBranchBackground",
        透明(&取(调色板, "error_bg"), 0.25),
    );
    颜色.放(
        "testing.message.error.badgeBackground",
        取(调色板, "error_bg"),
    );
    颜色.放("testing.message.error.badgeBorder", 取(调色板, "error_bg"));
    颜色.放(
        "testing.message.error.badgeForeground",
        取(调色板, "error_fg"),
    );
    颜色.放("testing.messagePeekBorder", 取(调色板, "error_bg"));
    颜色.放(
        "testing.messagePeekHeaderBackground",
        取(调色板, "error_bg"),
    );
    颜色.放("testing.iconPassed.retired", 取(调色板, "fg_disabled"));
    颜色.放("testing.iconFailed.retired", 取(调色板, "fg_disabled"));
    颜色.放("testing.iconErrored.retired", 取(调色板, "fg_disabled"));
    颜色.放("testing.iconQueued.retired", 取(调色板, "fg_disabled"));
    颜色.放("testing.iconUnset.retired", 取(调色板, "fg_disabled"));
    颜色.放("testing.iconSkipped.retired", 取(调色板, "fg_disabled"));
    // --- 列表 --------------------------------------------------------------
    颜色.放(
        "list.dropBetweenBackground",
        透明(&取(调色板, "accent_bg"), 0.75),
    );
    颜色.放(
        "list.filterMatchBackground",
        透明(&取(调色板, "accent_bg"), 0.25),
    );
    颜色.放("list.filterMatchBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "list.inactiveFocusBackground",
        取(调色板, "bg_active_sidebar"),
    );
    颜色.放("list.inactiveFocusOutline", 取(调色板, "border_dim"));
    // --- 缩略图 ---
    颜色.放(
        "minimap.chatEditHighlight",
        透明(&取(调色板, "accent_bg"), 0.4),
    );
    颜色.放("minimap.foregroundOpacity", "#000000c0");
    颜色.放("minimap.infoHighlight", 取(调色板, "accent_standalone"));
    // --- Notebook ---
    颜色.放("notebook.cellEditorBackground", 取(调色板, "editor_bg"));
    颜色.放(
        "notebook.focusedCellBackground",
        透明(&取(调色板, "accent_bg"), 0.08),
    );
    颜色.放(
        "notebook.inactiveFocusedCellBorder",
        取(调色板, "border_input"),
    );
    颜色.放(
        "notebook.symbolHighlightBackground",
        透明(&取(调色板, "accent_bg"), 0.15),
    );
    // --- 聊天 -----------------------------------------------------------------
    颜色.放("chat.requestBackground", 取(调色板, "bg_window"));
    颜色.放("chat.requestBorder", 取(调色板, "border"));
    颜色.放("chat.requestCodeBorder", 取(调色板, "border_input"));
    // --- 差异 ------------------------------------------------------------------
    颜色.放("diffEditor.insertedTextBorder", 取(调色板, "success_bg"));
    颜色.放("diffEditor.removedTextBorder", 取(调色板, "error_bg"));
    颜色.放("diffEditor.move.border", 取(调色板, "warning_bg"));
    颜色.放(
        "diffEditor.moveActive.border",
        取(调色板, "warning_standalone"),
    );
    // --- 扩展图标 ---
    颜色.放(
        "extensionIcon.preReleaseForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "extensionIcon.privateForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "extensionIcon.sponsorForeground",
        色调(调色板, "pink", 2, 4),
    );
    颜色.放(
        "extensionIcon.starForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "extensionIcon.verifiedForeground",
        取(调色板, "accent_standalone"),
    );
    // --- Markdown 提示块 ---
    颜色.放(
        "markdownAlert.note.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "markdownAlert.tip.foreground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "markdownAlert.important.foreground",
        色调(调色板, "purple", 1, 4),
    );
    颜色.放(
        "markdownAlert.warning.foreground",
        取(调色板, "warning_standalone"),
    );
    颜色.放(
        "markdownAlert.caution.foreground",
        取(调色板, "error_standalone"),
    );
    // --- 光标 / 操作列表 / 评论 ---
    颜色.放(
        "editorMultiCursor.primary.background",
        取(调色板, "editor_cursor"),
    );
    颜色.放(
        "editorMultiCursor.primary.foreground",
        取(调色板, "editor_bg"),
    );
    颜色.放(
        "editorMultiCursor.secondary.background",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorMultiCursor.secondary.foreground",
        取(调色板, "editor_bg"),
    );
    颜色.放("editorActionList.background", 取(调色板, "bg_popover"));
    颜色.放("editorActionList.foreground", 取(调色板, "fg_popover"));
    颜色.放(
        "editorActionList.focusBackground",
        表面叠加(调色板, &取(调色板, "accent_bg"), 0.25, "bg_popover"),
    );
    颜色.放("editorActionList.focusForeground", 取(调色板, "fg_popover"));
    颜色.放(
        "commentsView.resolvedIcon",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "commentsView.unresolvedIcon",
        取(调色板, "accent_standalone"),
    );
    // --- 交互窗口 / 终端 ---
    颜色.放("interactive.activeCodeBorder", 取(调色板, "accent_bg"));
    颜色.放("interactive.inactiveCodeBorder", 取(调色板, "border"));
    颜色.放("terminal.findMatchBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "terminal.findMatchHighlightBorder",
        取(调色板, "border_input"),
    );
    颜色.放(
        "terminal.initialHintForeground",
        取(调色板, "fg_disabled_view"),
    );
    // --- 智能体会话窗口 ---
    颜色.放(
        "agentsUpdateButton.downloadingBackground",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "agentsUpdateButton.downloadedBackground",
        取(调色板, "success_bg"),
    );
    颜色.放(
        "agentsMobileDiff.addedForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "agentsMobileDiff.deletedForeground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "agentsMobileDiff.modifiedForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放("activeSessionView.background", 取(调色板, "bg_card"));
    颜色.放("activeSessionView.foreground", 取(调色板, "fg_card"));
    颜色.放("inactiveSessionView.background", 取(调色板, "bg_window"));
    颜色.放("inactiveSessionView.foreground", 取(调色板, "fg_dim"));
    颜色.放(
        "agentFeedbackEditorWidget.background",
        取(调色板, "bg_popover"),
    );
    颜色.放("agentFeedbackEditorWidget.border", 取(调色板, "border"));
    // --- 图表 ---
    颜色.放("chart.axis", 取(调色板, "border"));
    颜色.放("chart.guide", 取(调色板, "border_dim"));
    颜色.放("chart.line", 取(调色板, "accent_standalone"));
    // --- 命令中心 / 复选框 / 徽章 ---
    颜色.放(
        "commandCenter.debuggingBackground",
        取(调色板, "warning_bg"),
    );
    颜色.放("commandCenter.inactiveBorder", 取(调色板, "border_dim"));
    颜色.放(
        "commandCenter.inactiveForeground",
        取(调色板, "fg_dim_headerbar"),
    );
    颜色.放(
        "checkbox.disabled.background",
        取(调色板, "bg_input_disabled"),
    );
    颜色.放(
        "checkbox.disabled.foreground",
        取(调色板, "fg_disabled_view"),
    );
    颜色.放("panelTitleBadge.background", 取(调色板, "accent_bg"));
    颜色.放("panelTitleBadge.foreground", 取(调色板, "accent_fg"));
    颜色.放("searchEditor.findMatchBackground", 取(调色板, "find_match"));
    颜色.放("searchEditor.findMatchBorder", 取(调色板, "accent_bg"));
    // --- 缩进参考线 / 装订线 / 合并 ---
    颜色.放("editorIndentGuide.background2", 取(调色板, "indent_guide"));
    颜色.放("editorIndentGuide.background3", 取(调色板, "indent_guide"));
    颜色.放("editorIndentGuide.background4", 取(调色板, "indent_guide"));
    颜色.放("editorIndentGuide.background5", 取(调色板, "indent_guide"));
    颜色.放("editorIndentGuide.background6", 取(调色板, "indent_guide"));
    颜色.放(
        "editorIndentGuide.activeBackground2",
        取(调色板, "indent_guide_active"),
    );
    颜色.放(
        "editorIndentGuide.activeBackground3",
        取(调色板, "indent_guide_active"),
    );
    颜色.放(
        "editorIndentGuide.activeBackground4",
        取(调色板, "indent_guide_active"),
    );
    颜色.放(
        "editorIndentGuide.activeBackground5",
        取(调色板, "indent_guide_active"),
    );
    颜色.放(
        "editorIndentGuide.activeBackground6",
        取(调色板, "indent_guide_active"),
    );
    颜色.放(
        "editorGutter.commentGlyphForeground",
        取(调色板, "success_standalone"),
    );
    颜色.放(
        "editorGutter.commentUnresolvedGlyphForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "editorGutter.commentDraftGlyphForeground",
        取(调色板, "warning_standalone"),
    );
    颜色.放("editorGutter.itemBackground", 取(调色板, "bg_hover_view"));
    颜色.放(
        "editorGutter.itemGlyphForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放("merge.border", 取(调色板, "border"));
    颜色.放(
        "merge.commonContentBackground",
        透明(&取(调色板, "fg_dim"), 0.25),
    );
    颜色.放(
        "merge.commonHeaderBackground",
        透明(&取(调色板, "fg_dim"), 0.4),
    );
    颜色.放(
        "merge.currentContentBackground",
        透明(&取(调色板, "success_bg"), 0.25),
    );
    颜色.放(
        "merge.currentHeaderBackground",
        透明(&取(调色板, "success_bg"), 0.4),
    );
    颜色.放(
        "merge.incomingContentBackground",
        透明(&取(调色板, "accent_bg"), 0.25),
    );
    颜色.放(
        "merge.incomingHeaderBackground",
        透明(&取(调色板, "accent_bg"), 0.4),
    );
    // --- 最后一轮：补齐注册表全部键 ---
    颜色.放(
        "agentFeedbackInputWidget.border",
        取(调色板, "border_input"),
    );
    颜色.放(
        "agentSessionReadIndicator.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放("agentSessionSelectedBadge.border", 取(调色板, "accent_bg"));
    颜色.放(
        "agentSessionSelectedUnfocusedBadge.border",
        取(调色板, "border_input"),
    );
    颜色.放(
        "aiCustomizationManagement.sashBorder",
        取(调色板, "border_dim"),
    );
    颜色.放("chatManagement.sashBorder", 取(调色板, "border_dim"));
    颜色.放(
        "debugIcon.stepBackForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "debugTokenExpression.type",
        取样式(&样式表, "def:type", true, &取(调色板, "fg")),
    );
    颜色.放("editor.border", 取(调色板, "border_dim"));
    颜色.放("editor.compositionBorder", 取(调色板, "accent_bg"));
    颜色.放("editor.findMatchBorder", 取(调色板, "accent_bg"));
    颜色.放(
        "editor.findMatchHighlightBorder",
        取(调色板, "border_input"),
    );
    颜色.放(
        "editor.findMatchHighlightForeground",
        取(调色板, "editor_search_match_fg"),
    );
    颜色.放("editor.findRangeHighlightBorder", "#00000000");
    颜色.放(
        "editor.focusedStackFrameHighlightBackground",
        透明(&取(调色板, "success_bg"), 0.25),
    );
    颜色.放(
        "editor.foldBackground",
        透明(&取(调色板, "accent_bg"), 0.15),
    );
    颜色.放(
        "editor.foldPlaceholderForeground",
        取(调色板, "fg_dim_view"),
    );
    颜色.放(
        "editor.inactiveLineHighlightBackground",
        透明(&取(调色板, "fg_view"), 0.04),
    );
    颜色.放(
        "editor.inlineValuesBackground",
        透明(&取(调色板, "accent_bg"), 0.15),
    );
    颜色.放("editor.inlineValuesForeground", 取(调色板, "fg_dim_view"));
    颜色.放(
        "editor.linkedEditingBackground",
        透明(&取(调色板, "accent_bg"), 0.2),
    );
    颜色.放(
        "editor.placeholder.foreground",
        取(调色板, "fg_placeholder"),
    );
    颜色.放("editor.selectionForeground", 取(调色板, "editor_fg"));
    颜色.放(
        "editor.stackFrameHighlightBackground",
        透明(&取(调色板, "warning_bg"), 0.25),
    );
    颜色.放(
        "editor.symbolHighlightBackground",
        透明(&取(调色板, "accent_bg"), 0.2),
    );
    颜色.放("editor.symbolHighlightBorder", 取(调色板, "accent_bg"));
    颜色.放("editor.wordHighlightBorder", "#00000000");
    颜色.放("editor.wordHighlightStrongBorder", "#00000000");
    颜色.放("editor.wordHighlightTextBorder", "#00000000");
    颜色.放("editorBracketMatch.foreground", 取(调色板, "fg_view"));
    颜色.放("editorError.border", 取(调色板, "error_bg"));
    颜色.放("editorWarning.border", 取(调色板, "warning_bg"));
    颜色.放("editorInfo.border", 取(调色板, "accent_bg"));
    颜色.放("editorGhostText.border", "#00000000");
    颜色.放(
        "editorGroup.dropIntoPromptBorder",
        取(调色板, "border_input"),
    );
    颜色.放(
        "editorMarkerNavigationError.headerBackground",
        取(调色板, "error_bg"),
    );
    颜色.放(
        "editorMarkerNavigationWarning.headerBackground",
        取(调色板, "warning_bg"),
    );
    颜色.放(
        "editorMarkerNavigationInfo.headerBackground",
        取(调色板, "accent_bg"),
    );
    颜色.放(
        "editorMinimap.inlineChatInserted",
        取(调色板, "success_standalone"),
    );
    颜色.放("editorStickyScrollGutter.background", 取(调色板, "bg_card"));
    颜色.放("editorUnnecessaryCode.opacity", "#000000c0");
    颜色.放("mcpIcon.starForeground", 取(调色板, "warning_standalone"));
    颜色.放(
        "mergeEditor.conflictingLines.background",
        透明(&取(调色板, "warning_bg"), 0.2),
    );
    颜色.放("modernActivityBar.background", 取(调色板, "bg_sidebar"));
    颜色.放(
        "modernActivityBar.inactiveBackground",
        取(调色板, "bg_sidebar_backdrop"),
    );
    颜色.放("modernPanel.border", 取(调色板, "border"));
    颜色.放("modernSash.gripForeground", 取(调色板, "fg_disabled"));
    颜色.放(
        "notebookEditorOverviewRuler.runningCellForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "notebookScrollbarSlider.background",
        取(调色板, "scrollbar"),
    );
    颜色.放(
        "notebookScrollbarSlider.hoverBackground",
        取(调色板, "scrollbar_hover"),
    );
    颜色.放(
        "notebookScrollbarSlider.activeBackground",
        取(调色板, "scrollbar_active"),
    );
    颜色.放(
        "notebookStatusErrorIcon.foreground",
        取(调色板, "error_standalone"),
    );
    颜色.放(
        "notebookStatusRunningIcon.foreground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "notebookStatusSuccessIcon.foreground",
        取(调色板, "success_standalone"),
    );
    颜色.放("outputView.background", 取(调色板, "bg_window"));
    颜色.放("outputViewStickyScroll.background", 取(调色板, "bg_card"));
    颜色.放(
        "peekViewEditorStickyScroll.background",
        取(调色板, "bg_card"),
    );
    颜色.放(
        "peekViewEditorStickyScrollGutter.background",
        取(调色板, "bg_card"),
    );
    颜色.放("profiles.sashBorder", 取(调色板, "border_dim"));
    颜色.放("scrollbar.background", "#00000000");
    颜色.放("search.resultsInfoForeground", 取(调色板, "fg_dim_view"));
    颜色.放("sideBarStickyScroll.background", 取(调色板, "bg_card"));
    颜色.放("sideBarStickyScroll.border", 取(调色板, "border"));
    颜色.放("sideBarTitle.background", 取(调色板, "bg_sidebar"));
    颜色.放(
        "sideBarTitle.border",
        if 调色板.high_contrast {
            取(调色板, "border")
        } else {
            "#00000000".to_string()
        },
    );
    颜色.放(
        "sideBySideEditor.horizontalBorder",
        取(调色板, "border_dim"),
    );
    颜色.放("sideBySideEditor.verticalBorder", 取(调色板, "border_dim"));
    颜色.放("simpleFindWidget.sashBorder", 取(调色板, "border_dim"));
    颜色.放(
        "tab.unfocusedInactiveModifiedBorder",
        取(调色板, "border_dim"),
    );
    颜色.放(
        "terminalCommandGuide.foreground",
        取(调色板, "border_input"),
    );
    颜色.放("terminalOverviewRuler.border", "#00000000");
    颜色.放(
        "terminalOverviewRuler.cursorForeground",
        取(调色板, "accent_standalone"),
    );
    颜色.放(
        "terminalOverviewRuler.findMatchForeground",
        取(调色板, "find_match"),
    );
    颜色.放("terminalStickyScroll.background", 取(调色板, "bg_card"));
    颜色.放("terminalStickyScroll.border", 取(调色板, "border"));
    颜色.放(
        "terminalStickyScrollHover.background",
        取(调色板, "bg_hover_card"),
    );
    颜色.放("textPreformat.border", 取(调色板, "border_input"));
    颜色.放("toolbar.hoverOutline", 取(调色板, "border_input"));
    颜色.放("walkthrough.stepTitle.foreground", 取(调色板, "fg"));

    if 彩色状态栏 {
        颜色.覆盖("statusBar.background", 取(调色板, "accent_bg"));
        颜色.覆盖("statusBar.foreground", 取(调色板, "accent_fg"));
        颜色.覆盖("statusBarItem.hoverBackground", 取(调色板, "accent_hover"));
        颜色.覆盖("statusBarItem.hoverForeground", 取(调色板, "accent_fg"));
        颜色.覆盖(
            "statusBarItem.activeBackground",
            取(调色板, "accent_active"),
        );
        颜色.覆盖(
            "statusBarItem.prominentBackground",
            取(调色板, "accent_hover"),
        );
        颜色.覆盖("statusBarItem.prominentForeground", 取(调色板, "accent_fg"));
        颜色.覆盖(
            "statusBarItem.prominentHoverBackground",
            取(调色板, "accent_active"),
        );
        颜色.覆盖("statusBar.debuggingBackground", 取(调色板, "warning_bg"));
        颜色.覆盖("statusBar.debuggingForeground", 取(调色板, "warning_fg"));
        颜色.覆盖("statusBar.noFolderBackground", 取(调色板, "accent_active"));
        颜色.覆盖("statusBar.noFolderForeground", 取(调色板, "accent_fg"));
        颜色.覆盖("statusBarItem.remoteBackground", 取(调色板, "success_bg"));
        颜色.覆盖("statusBarItem.remoteForeground", 取(调色板, "success_fg"));
    }

    Ok(颜色.完成())
}

/// 读取调色板角色；缺失角色属于不变量错误，与 Python 的 KeyError 一致。
fn 取(p: &调色板对象, 角色: &str) -> String {
    p.取(角色).to_string()
}

/// 在颜色原有透明度上乘以权重；失败属于不变量错误。
fn 透明(颜色: &str, 权重: f64) -> String {
    合成透明颜色(颜色, 权重).expect("内部颜色运算")
}

/// 把颜色的权重比例合成到具名表面；失败属于不变量错误。
fn 表面叠加(p: &调色板对象, 颜色: &str, 权重: f64, 表面: &str) -> String {
    p.表面叠加(颜色, 权重, 表面).expect("内部颜色运算")
}

/// 对应 Python 的 `sy`：样式缺失、字段为 None 或空字符串时使用回退色。
fn 取样式(
    样式表: &BTreeMap<String, 样式信息>, 名称: &str, 取前景: bool, 回退: &str
) -> String {
    样式表
        .get(名称)
        .and_then(|样式| {
            if 取前景 {
                样式.foreground.clone()
            } else {
                样式.background.clone()
            }
        })
        .filter(|值| !值.is_empty())
        .unwrap_or_else(|| 回退.to_string())
}

/// 对应 Python 的 `hue`：按明暗模式取 GNOME 色阶（级数从 1 开始）。
fn 色调(p: &调色板对象, 名称: &str, 深色级: usize, 浅色级: usize) -> String {
    let 级 = if p.mode == "dark" {
        深色级
    } else {
        浅色级
    };
    色阶(名称).expect("未知色阶")[级 - 1].to_string()
}

/// 收集颜色条目；None 表示普通主题中不定义的键，最后统一过滤。
struct 颜色表 {
    条目: Vec<(&'static str, Option<String>)>,
}

impl 颜色表 {
    fn 新() -> Self {
        Self { 条目: Vec::new() }
    }

    fn 放(&mut self, 键: &'static str, 值: impl Into<String>) {
        self.条目.push((键, Some(值.into())));
    }

    fn 放可选(&mut self, 键: &'static str, 值: Option<String>) {
        self.条目.push((键, 值));
    }

    /// 对应 Python 的 `dict.update`：已有键原位替换，缺失键追加到末尾。
    fn 覆盖(&mut self, 键: &'static str, 值: impl Into<String>) {
        let 值 = 值.into();
        if let Some(项) = self.条目.iter_mut().find(|(已有, _)| *已有 == 键) {
            项.1 = Some(值);
        } else {
            self.条目.push((键, Some(值)));
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
}
