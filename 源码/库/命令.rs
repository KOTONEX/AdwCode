// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 命令行解析与子命令分发；子命令、参数与提示使用简体中文。

use crate::错误::{工具错误, 结果};

const 帮助文本: &str = "\
AdwCode 构建与打包工具

用法：adwcode <子命令> [参数]

子命令：
  主题                 生成主题并同步 package.json（--监视 添加调试标记）
  校验                 校验已生成的主题、产品图标与颜色键
  检查样式             对照已安装的 VS Code 检查自定义 CSS（--样式表 路径）
  打包                 生成 VSIX（--变更日志 路径 供无 Git 副本使用）
  变更日志             生成 CHANGELOG.md
  发布说明             生成发布说明（--标签 vX.Y.Z）
  更新默认数据         刷新随附的 VS Code 数据与颜色键表
  性能基准             运行离线性能基准
  生成自有字形         生成自有产品图标字体
  生成导入字形         生成导入产品图标字体与预览
  检查                 运行静态检查、类型检查与离线测试
  格式化               格式化全部源码
  帮助                 显示本说明
";

/// 执行命令行；返回 `Ok(())` 表示成功。
pub fn 执行(参数: &[String]) -> 结果<()> {
    let Some((子命令, 其余)) = 参数.split_first() else {
        print!("{帮助文本}");
        return Ok(());
    };
    match 子命令.as_str() {
        "帮助" | "--帮助" | "-h" => {
            print!("{帮助文本}");
            Ok(())
        }
        "主题" => crate::主题生成::生成入口(&crate::仓库::根目录()?, 其余),
        "校验" => crate::主题生成::校验入口(&crate::仓库::根目录()?),
        "变更日志" => crate::变更日志::入口(&crate::仓库::根目录()?, false, 其余),
        "发布说明" => crate::变更日志::入口(&crate::仓库::根目录()?, true, 其余),
        其他 => Err(工具错误::新(format!(
            "子命令尚未迁移：{其他}；运行 adwcode 帮助 查看用法"
        ))),
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 无参数时打印帮助() {
        assert!(执行(&[]).is_ok());
    }

    #[test]
    fn 未知子命令报错() {
        let 参数 = vec!["不存在".to_string()];
        let 错误 = 执行(&参数).unwrap_err();
        assert!(错误.消息.contains("子命令尚未迁移：不存在"));
    }
}
