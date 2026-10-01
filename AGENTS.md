# AGENTS.md

给自动化代理（含 AI）的项目说明；人类贡献者请看 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 项目一句话

VS Code 的 Adwaita 主题，由 **libadwaita 1.10**（GNOME 51）的取值生成。
目标是让 VS Code 看起来像原生 GNOME 应用（尤其是 GNOME Builder）。

需要 Python 3.9+（代码中的 `X | Y` 注解均依赖 `from __future__ import annotations`）；
CI 与本地开发使用最新稳定版 3.14。

## 目录

- `src/palette.py` —— libadwaita 1.10 变量、强调色、颜色合成工具
- `src/mapping.py` —— VS Code 颜色键到 Adwaita 角色的映射
- `src/tokens.py` —— GtkSourceView 样式名到 TextMate 作用域的映射
- `src/build.py` / `src/package.py` / `src/update_defaults.py`
- `tests/test_adwcode.py` —— 离线单元测试，运行 `make test`
- `src/gtksourceview_xml/` —— 随附的 GtkSourceView 方案（LGPL-2.1+）
- `src/vscode_defaults/` —— 随附的 VS Code 默认数据与键表（MIT）
- `themes/` —— 生成的主题 JSON，已提交，便于符号链接安装从克隆即可使用
- `product-icons/`、`extras/`、`extension/`
- `types/`、`mypy.ini`、`tsconfig.json` —— 类型检查配置与手写最小类型面
- `docs/01-第三方许可证.md` —— 第三方登记；`Makefile` 是统一命令入口

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
- `extras/*.css` 通过「Custom CSS and JS Loader」扩展生效。VS Code 1.139 在约
  250 处引用 `--vscode-cornerRadius-*` / `--vscode-spacing-*` 却从未定义它们，
  因此 `gnome-look.css` 自行定义这些令牌以提供 Adwaita 几何。VS Code 升级后请
  运行 `check_css.py`：它会校验每个类选择器仍存在于已安装的构建中（由
  JavaScript 创建的类名会在 bundle 中搜索）。
- 版本号唯一事实源是 `package.json` 的 `version`，发布流程见
  `.github/workflows/release.yml`。
- 提交信息、代码注释与 README 使用简体中文。

## 常用命令

- 静态检查：`make lint`
- 类型检查：`make typecheck`（mypy 严格检查 `src/`、`tests/`；tsc 检查
  `extension/extension.js`，缺工具时跳过，CI 以 `ADWCODE_TYPECHECK_STRICT=1` 强制）
- 校验（键覆盖、未知键、对比度、产品图标）：`make check`
- 离线单元测试：`make test`
- 构建主题并同步 `package.json`：`make build`（等价于 `python3 src/build.py`）
- 生成全部九种强调色：`python3 src/build.py --accents all`
- 对照已安装的 VS Code 检查自定义 CSS：`python3 src/check_css.py`
- 打包 VSIX（无需 Node.js）：`make package`
- 刷新随附的 VS Code 数据与键表：`python3 src/update_defaults.py`
- 开发时主题 JSON 即时重载：`python3 src/build.py --watch`
- 发布：推送 `v*` 标签后由 GitHub Actions 自动构建并上传 VSIX
  （`.github/workflows/release.yml`，会先校验标签与版本一致、产物与提交一致，
  再跑静态检查与单元测试）；配置仓库 Secrets `VSCE_PAT`、`OVSX_PAT` 后，同一
  VSIX 还会发布到 VS Code 扩展市场与 Open VSX（未配置时自动跳过）

任何改动完成前都要跑 `make lint && make check && make test`。

## 提交

提交信息使用**简体中文类型前缀**（`新增:` / `修复:` / `文档:` / `测试:` / `重构:` /
`杂务:` / `初始化:`，格式为 `类型: 描述`），完整约定见
[CONTRIBUTING.md](CONTRIBUTING.md)。
