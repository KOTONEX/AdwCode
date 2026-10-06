// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 统一的工具错误类型：面向命令行输出的简体中文消息。

use std::fmt;

/// 工具错误；`来源` 保留底层 IO 错误用于诊断。
#[derive(Debug)]
pub struct 工具错误 {
    pub 消息: String,
    pub 来源: Option<std::io::Error>,
}

impl 工具错误 {
    #[must_use]
    pub fn 新(消息: impl Into<String>) -> Self {
        Self {
            消息: 消息.into(),
            来源: None,
        }
    }

    #[must_use]
    pub fn 带来源(消息: impl Into<String>, 来源: std::io::Error) -> Self {
        Self {
            消息: 消息.into(),
            来源: Some(来源),
        }
    }
}

impl fmt::Display for 工具错误 {
    fn fmt(&self, 输出: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(输出, "{}", self.消息)
    }
}

impl std::error::Error for 工具错误 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.来源
            .as_ref()
            .map(|来源| 来源 as &(dyn std::error::Error + 'static))
    }
}

impl From<std::io::Error> for 工具错误 {
    fn from(来源: std::io::Error) -> Self {
        Self {
            消息: 来源.to_string(),
            来源: Some(来源),
        }
    }
}

pub type 结果<T> = std::result::Result<T, 工具错误>;
