# AGENTS.md

给自动化代理（含 AI）的项目说明；人类贡献者请看 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 新会话入口

1. 先运行 `git status --short`，区分已有改动、暂存内容与未跟踪文件；不要覆盖或
   顺手提交与任务无关的内容。版本读取 `package.json`，提交状态读取 Git。
2. 阅读本文件；按任务查阅 [架构与状态](docs/03-架构与实现状态.md)、
   [开发与验证](docs/04-开发验证与发布.md)。若工作区存在本地待办文档
   `docs/02-下一主版本候选.md`，可按需查阅；不要把候选功能当成已实现功能。
3. 完成任务后说明改动范围、验证结果和已知限制。涉及重载、推送或发布时，
   遵循当前任务的授权范围。

## 自动化开发约束

- 窗口重载会中断正在运行的扩展、调试任务与智能体会话。自动化测试不得直接执行
  `workbench.action.reloadWindow` 或加载器的启用、重新加载命令。
- 修改受监视文件前检查 `adwcode.autoReload`；若开启，先说明影响并关闭该设置，
  防止文件修改间接触发重载。不要静默恢复开启状态。未知配置位置时先查找，
  不把某次会话的路径当成通用路径。
- 推荐设置会开启自动重载，配置项默认值则为关闭。修改该行为时同步更新文档
  与测试；开发期间可关闭自动重载，避免文件变化中断工作。
- 汉堡菜单保留 VS Code 原生位置。`compact` 在活动栏显示按钮，不添加移动
  菜单的脚本，不用 CSS 把它强行挪到顶栏。
- 外观以 GNOME HIG、libadwaita 1.10 和 GNOME Builder 为参照，当前侧重外观
  完整度，并同时考虑布局、状态、密度和键盘操作。
- 提交信息、README、文档、代码注释、界面提示与自有脚本说明使用简体中文。
  API、配置键、文件名、专有名称、第三方命令名称与许可证原文保留原文。
- Git 身份先查本机配置；必要时核对本机 gh 账号与历史提交。不要猜测身份，
  不修改全局 Git 配置。提交、推送、打标签和发布是分别授权的动作。

## 项目一句话

VS Code 的 Adwaita 主题，由 **libadwaita 1.10**（GNOME 51）的取值生成。
目标是让 VS Code 看起来像原生 GNOME 应用（尤其是 GNOME Builder）。

需要 Python 3.9+（代码中的 `X | Y` 注解均依赖 `from __future__ import annotations`）；
CI 与本地开发使用 Python 3.14 自由线程版本（GIL 关闭）。

## 目录

- `src/palette.py` —— libadwaita 1.10 变量、强调色、颜色合成工具
- `src/mapping.py` —— VS Code 颜色键到 Adwaita 角色的映射
- `src/tokens.py` —— GtkSourceView 样式名到 TextMate 作用域的映射
- `src/build.py` / `src/package.py` / `src/update_defaults.py`
- `tests/test_adwcode.py` —— 离线单元测试，运行 `meson test -C builddir --print-errorlogs`
- `src/gtksourceview_xml/` —— 随附的 GtkSourceView 方案（LGPL-2.1+）
- `src/vscode_defaults/` —— 随附的 VS Code 默认数据与键表（MIT）
- `themes/` —— 生成的主题 JSON，已提交，便于符号链接安装从克隆即可使用
- `product-icons/`、`extras/`、`extension/`
- `types/`、`ty.toml`、`tsconfig.json` —— 类型检查配置与手写最小类型面
- `docs/01-第三方许可证.md` —— 第三方登记；`meson.build` 是统一命令入口

## 约定

- 主题 JSON 中的颜色必须是十六进制（`#rrggbb` / `#rrggbbaa`）：VS Code 会忽略
  CSS Color 4 写法（如 `rgb(0 0 6 / 36%)`）。请使用 `palette.as_hex()`。
- `contrastBorder` / `contrastActiveBorder` 只属于高对比度主题；在普通主题中定义
  它们会到处多出描边（VS Code 的 CSS 以 `unset` / `transparent` 作为回退）。
- 新增颜色键必须存在于 `src/vscode_defaults/registry_keys.json`，或在
  `build.LEGACY_KEYS` 中，否则 `--check` 会失败。
- 扩展为无构建步骤、无依赖的纯 JavaScript；JS 类型检查由 `// @ts-check` +
  `types/` 手写最小类型面提供，不引入 `@types` 依赖。
- `themes/` 是生成产物，不要手工编辑。
- `extras/*.css` 通过「Custom CSS and JS Loader」扩展生效。VS Code 1.140 在约
  250 处引用 `--vscode-cornerRadius-*` / `--vscode-spacing-*` 却从未定义它们，
  因此 `gnome-look.css` 自行定义这些令牌以提供 Adwaita 几何。VS Code 升级后请
  运行 `check_css.py`：它会校验每个类选择器仍存在于已安装的构建中（由
  JavaScript 创建的类名会在 bundle 中搜索）。
- 版本号唯一事实源是 `package.json` 的 `version`，发布流程见
  `.github/workflows/release.yml`。
- 不以“类名存在”或“磁盘补丁已更新”宣称当前窗口外观已验证。

## 常用命令

默认解释器为 `python3.14t`，必须是 GIL 关闭的 Python 3.14 自由线程版本；
可通过 `-Dpython=/绝对路径/python3.14t` 指定。首次运行 `meson setup builddir`；后续可用 `meson setup --reconfigure builddir` 更新配置。

- 静态检查：`meson compile -C builddir lint`
- 类型检查：`meson compile -C builddir typecheck`（ty 检查 `src/`、`tests/`；tsc 检查
  `extension/extension.js`，缺少 ty、tsc 或 Node.js 时失败）
- 校验（键覆盖、未知键、对比度、产品图标）：`meson compile -C builddir check`
- 完整检查（静态、类型、主题与离线单元测试）：`meson test -C builddir --print-errorlogs`
- 构建主题并同步 `package.json`：`meson compile -C builddir themes`（等价于 `python3.14t src/build.py`）
- 生成全部九种强调色：`python3.14t src/build.py --accents all`
- 对照已安装的 VS Code 检查自定义 CSS：`python3.14t src/check_css.py`
- 打包 VSIX（无需 Node.js）：`meson compile -C builddir package`
- 刷新随附的 VS Code 数据与键表：`python3.14t src/update_defaults.py`
- 开发时主题 JSON 即时重载：`python3.14t src/build.py --watch`
- 发布：推送 `v*` 标签后由 GitHub Actions 自动构建并上传 VSIX
  （`.github/workflows/release.yml`，会先校验标签与版本一致、产物与提交一致，
  再跑静态检查与单元测试）；配置仓库 Secrets `VSCE_PAT`、`OVSX_PAT` 后，同一
  VSIX 还会发布到 VS Code 扩展市场与 Open VSX（未配置时自动跳过）

任何改动完成前都要跑 `meson test -C builddir --print-errorlogs`。

## 提交

提交信息使用**简体中文类型前缀**（`新增:` / `修复:` / `文档:` / `测试:` / `重构:` /
`杂务:` / `初始化:`，格式为 `类型: 描述`），完整约定见
[CONTRIBUTING.md](CONTRIBUTING.md)。
