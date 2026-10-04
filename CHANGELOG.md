# 更新日志

本项目的所有重要变更都记录在此文件。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)；
提交信息使用中文类型前缀（见 [CONTRIBUTING.md](CONTRIBUTING.md)）。

## [未发布]

（暂无）

## [1.1.0] - 2026-10-05

- 修复直接加载仓库 CSS 时的状态识别，并限定补丁检测范围；加载器更新失败
  或关闭自动重载后取消窗口重载，停用扩展时清理监视器和待执行任务。
- 高对比度主题保留更清晰的滚动条配色，VSIX 补齐 CSS 文件类型声明。
- 统一侧栏、面板、编辑器与通知工具栏的按钮圆角和键盘焦点环，补充小控件、
  设置行与对话框外观，并避免通知按钮的焦点环被操作区裁切。
- 新增“查看外观安装状态”面板，分别检查加载器、安装副本、imports 配置和
  磁盘补丁，支持刷新，不自动重载窗口。
- 扩大命令面板、通知中心、补全详情与悬浮提示的圆角和阴影适配，统一搜索和
  设置页复合输入框的圆角与键盘焦点环。
- 新增开发用设置 `adwcode.autoReload`：监视 `extras/*.css`、`extras/*.js`、`themes/*.json`、
  清单与扩展代码，变化后自动重新应用 Custom CSS 并重载窗口。
- 默认使用折叠菜单（`window.menuBarVisibility: compact`），按钮保留在活动栏，
  不修改菜单位置；可在设置中改回 `classic` 恢复完整菜单栏。
- 主题几何进一步对齐 libadwaita 1.10：9px 按钮/输入框/列表行/标签页、
  15px 弹出层与快速输入、Adwaita 阴影、内缩细滚动条滑块，并为按钮与
  下拉框加上 GNOME 式 2px 强调色焦点环。
- 滚动条仅在横截面内缩，高对比度保留完整滑块面积。

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
