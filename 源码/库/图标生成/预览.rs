// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

use super::几何::{子路径, 字体变换};
use super::字符串字段;
use crate::错误::{工具错误, 结果};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 导出陈列预览并清理过期文件。
pub(super) fn 导出预览(
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
    let 许可 = 许可.replace("--", "- -");
    let 署名 = 署名.replace("--", "- -");
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
