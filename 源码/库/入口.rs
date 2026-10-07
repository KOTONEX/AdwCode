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

#[path = "发布策略.rs"]
pub mod 发布策略;

#[path = "发布上传.rs"]
pub mod 发布上传;

#[path = "验证设置.rs"]
pub mod 验证设置;

#[path = "打包.rs"]
pub mod 打包;

#[path = "检查样式.rs"]
pub mod 检查样式;

#[path = "更新默认数据.rs"]
pub mod 更新默认数据;

#[path = "图标生成/mod.rs"]
pub mod 图标生成;

#[path = "仓库.rs"]
pub mod 仓库;

#[path = "命令.rs"]
pub mod 命令;

#[path = "摘要.rs"]
pub mod 摘要;

#[path = "性能基准.rs"]
pub mod 性能基准;

#[path = "工作台基准.rs"]
pub mod 工作台基准;

#[path = "检查.rs"]
pub mod 检查;

#[path = "文件事务.rs"]
pub mod 文件事务;

#[path = "宿主测试.rs"]
pub mod 宿主测试;

#[path = "运行工具.rs"]
pub mod 运行工具;
