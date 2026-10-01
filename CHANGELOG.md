# 更新日志

本项目的所有重要变更都记录在此文件。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)；
提交信息使用中文类型前缀（见 [CONTRIBUTING.md](CONTRIBUTING.md)）。

## [未发布]

（暂无）

## [1.0.0] - 2026-10-01

- 首个正式版本，提交历史自本版本起为单个初始提交。
- 主题取值自 libadwaita 1.10（GNOME 51）：深浅两套配色、高对比度与九种强调色；
  扩展可跟随 `org.gnome.desktop.interface accent-color` 自动切换变体。
- 语法高亮对齐 GNOME Builder 的 GtkSourceView 方案（含 `semanticTokenColors`），
  另提供使用 VS Code 自带 token 颜色的「默认语法高亮」变体与彩色状态栏变体。
- 产品图标主题替换窗口控制字形；`extras/` 下的 CSS 经「Custom CSS and JS Loader」
  提供 Adwaita 几何（圆角、间距、阴影）与「仅关闭按钮」控件样式。
- 一键推荐设置（对齐 GNOME Builder 布局）等命令；扩展无构建步骤、无运行时依赖。
- Python 源码与测试带类型注解（`mypy --strict`），扩展以 JSDoc + `types/` 手写
  最小类型面经 tsc 检查：`make typecheck`，CI 以 `ADWCODE_TYPECHECK_STRICT=1` 强制。
- 发布链路：推送 `v*` 标签即自动校验、测试、打包并上传 GitHub Release，并在配置
  仓库 Secrets `VSCE_PAT`、`OVSX_PAT` 后发布到 VS Code 扩展市场与 Open VSX。
