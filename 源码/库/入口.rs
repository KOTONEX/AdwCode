// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! AdwCode 构建与打包工具库。
//!
//! 非 ASCII 模块名必须用 `#[path]` 显式声明；子模块文件名使用简体中文。

#[path = "错误.rs"]
pub mod 错误;

#[path = "有序映射.rs"]
pub mod 有序映射;

#[path = "调色板.rs"]
pub mod 调色板;

#[path = "语法映射.rs"]
pub mod 语法映射;

#[path = "界面映射.rs"]
pub mod 界面映射;

#[path = "主题生成.rs"]
pub mod 主题生成;

#[path = "变更日志.rs"]
pub mod 变更日志;

#[path = "打包.rs"]
pub mod 打包;

#[path = "仓库.rs"]
pub mod 仓库;

#[path = "命令.rs"]
pub mod 命令;
