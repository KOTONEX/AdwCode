# AGENTS.md

给自动化代理（含 AI）的项目说明；人类贡献者请看 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 新会话入口

1. 先运行 `git status --short`，区分已有改动、暂存内容与未跟踪文件；不要覆盖或
   顺手提交与任务无关的内容。版本读取 `package.json`，提交状态读取 Git。
2. 阅读本文件；按任务查阅 [架构与状态](文档/03-架构与实现状态.md)、
   [开发与验证](文档/04-开发验证与发布.md)。后续功能候选见
   [待办清单](文档/02-待办清单.md)，按需查阅；实现与审查记录见
   [外观实现与验收](文档/05-外观实现与验收.md)，不要把其他候选功能当成已实现功能。
3. 完成任务后说明改动范围、验证结果和已知限制。涉及重载、推送或发布时，
   遵循当前任务的授权范围。

## 自动化开发约束

- 窗口重载会中断正在运行的扩展、调试任务与智能体会话。自动化测试不得直接执行
  `workbench.action.reloadWindow` 或加载器的启用、重新加载命令。
- 扩展不监视文件，也不会自动重载；外观或代码改动需要生效时，由操作者手动重载窗口。
- 汉堡菜单保留 VS Code 原生位置。`compact` 在活动栏显示按钮，不添加移动
  菜单的脚本，不用 CSS 把它强行挪到顶栏。
- 产品图标选材优先级为 Adwaita、GNOME Builder、MoreWaita；按语义匹配选用，
  不为更换来源而丢失状态区别。来源记录和图标陈列同步更新。
- 外观以 GNOME HIG、libadwaita 1.10 和 GNOME Builder 为参照，当前侧重外观
  完整度，并同时考虑布局、状态、密度和键盘操作。
- 提交信息、README、文档、代码注释、界面提示与自有脚本说明使用简体中文。
  项目自有 API、配置键、命令标识、脚本参数、文件名和目录名除明确特例外，必须采用简体中文；
  特例范围见下文“中文接口”，不得将 API、配置键、文件名或目录名整体视为免于汉化。
- 自有代码采用 `AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later`，自有资产采用
  `AGPL-3.0-or-later OR CC-BY-SA-4.0+`；范围见 `许可声明.md`。第三方及其衍生
  内容保留原许可；特别注意 `源码/库/语法映射.rs` 的 GPL 映射，不将混合文件整体重新许可。
- 本项目的主题标签、产品图标主题与设置分组使用 AdwCode，主题和图标映射文件
  使用 `adwcode` 前缀；上游项目、字体、配色方案与图标来源标识保留原名。
- Git 身份先查本机配置；必要时核对本机 gh 账号与历史提交。不要猜测身份，
  不修改全局 Git 配置。提交、推送、打标签和发布是分别授权的动作。

## 项目一句话

VS Code 的 AdwCode 主题，由 **libadwaita 1.10**（GNOME 51）的取值生成。
目标是让 VS Code 看起来像原生 GNOME 应用（尤其是 GNOME Builder）。

需要 Rust 1.99（`rust-toolchain.toml` 固定，含 clippy 与 rustfmt）；扩展 JS 的语法与
类型检查、离线测试需要 Node.js 与 TypeScript 编译器。

## 目录

- `源码/库/调色板.rs` —— libadwaita 1.10 变量、强调色、颜色合成工具
- `源码/库/界面映射.rs` —— VS Code 颜色键到 Adwaita 角色的映射
- `源码/库/语法映射.rs` —— GtkSourceView 样式名到 TextMate 作用域的映射
- `源码/库/主题生成.rs` / `源码/库/打包.rs` / `源码/库/更新默认数据.rs`
- `源码/库/变更日志.rs` —— 从 Git 版本标签和提交标题生成变更日志与发布说明
- `源码/库/发布上传.rs` —— 本地与 CI 共用的 Release 附件/市场发布编排
- `源码/库/发布策略.rs`、`源码/库/验证设置.rs` —— 发布权限与清单设置验证，回归由 Cargo 执行
- `源码/库/命令.rs`、`源码/程序/主程序.rs` —— `adwcode` 子命令与二进制入口
- 单元测试与模块内联，运行 `cargo test`
- `源码/GtkSourceView方案/` —— 随附的 GtkSourceView 方案（LGPL-2.1+）
- `源码/VSCode默认数据/` —— 随附的 VS Code 默认数据与键表（MIT）
- `主题/` —— 生成的主题 JSON，已提交，便于符号链接安装从克隆即可使用
- `资产/` —— 原创扩展图标 PNG、来源与设计说明；制作方式见 `资产/README.md`
- `产品图标/`、`附加外观/`、`扩展/`；`附加外观/窗口状态.js` 只同步窗口状态
- `类型声明/`、`tsconfig.json` —— 扩展 JS 的类型检查配置与手写最小类型面
- `Cargo.toml`、`rust-toolchain.toml` —— Rust 工作区与固定工具链
- `基准/` —— Linux 性能基准，运行方式见其 README，测量结果见
  [性能测试](文档/07-性能测试.md)
- `文档/01-第三方许可证.md` —— 第三方登记；`adwcode` 是统一命令入口
- `文档/06-产品图标陈列.md` —— 实际使用的字形、缩略图与标识；图标映射变化时同步更新

## 约定

- 主题 JSON 中的颜色必须是十六进制（`#rrggbb` / `#rrggbbaa`）：VS Code 会忽略
  CSS Color 4 写法（如 `rgb(0 0 6 / 36%)`）。请使用 `调色板::规范颜色格式`。
- `contrastBorder` / `contrastActiveBorder` 只属于高对比度主题；在普通主题中定义
  它们会到处多出描边（VS Code 的 CSS 以 `unset` / `transparent` 作为回退）。
- 新增颜色键必须存在于 `源码/VSCode默认数据/registry_keys.json`，或在
  `主题生成::补充颜色键` 中，否则 `校验` 会失败。
- 扩展为无构建步骤、无依赖的纯 JavaScript；模块工厂见 `扩展/外观服务.js`，入口只装配与注册；JS 类型检查由 `// @ts-check` +
  `类型声明/` 手写最小类型面提供，不引入 `@types` 依赖。
- `主题/` 是生成产物，不要手工编辑。
- `附加外观/*.css` 和 `附加外观/窗口状态.js` 通过「Custom CSS and JS Loader」扩展生效。
  非活动状态由脚本同步到导航容器，不恢复工作台祖先上的 `:has()` 规则；
  安装命令把已知源码与副本引用统一为一份副本，保留其他用户加载项。
  VS Code 1.140 在约 250 处引用 `--vscode-cornerRadius-*` / `--vscode-spacing-*`
  却从未定义它们，因此 `GNOME外观.css` 自行定义这些令牌以提供 Adwaita 几何。
  VS Code 升级后请
  运行 `cargo run --quiet -- 检查样式`：它会校验每个类选择器仍存在于已安装的构建中（由
  JavaScript 创建的类名会在 bundle 中搜索；项目自有状态类核对脚本的显式创建操作）。
- VSIX 产物不变的改动只提交与推送，不更新发布版本号、不打版本标签、不创建 Release。
  是否发布以实际打包内容的变化为依据，不因产生新提交而自动发布。
- 版本号唯一事实源是 `package.json` 的 `version`，发布流程见
  `.github/workflows/release.yml`。
- 不手工维护 `CHANGELOG.md`；提交标题与主线上的版本标签是日志输入。
  `cargo run --quiet -- 变更日志` 生成 `builddir/CHANGELOG.md`，打包时自动写入
  VSIX。完整历史和标签不可缺失，CI 必须完整检出；未提交修改不会进入日志。
- 不以“类名存在”或“磁盘补丁已更新”宣称当前窗口外观已验证。

## 常用命令

Rust 工具链由 `rust-toolchain.toml` 固定；首次运行 `cargo build`。命令统一由
`adwcode` 提供，开发期用 `cargo run --quiet -- <子命令>`，发布产物用
`./target/release/adwcode <子命令>`。

- 完整检查（cargo fmt/clippy/test、校验、Node/tsc 与离线 JS 测试）：`cargo run --quiet -- 检查`
- 格式化全部 Rust 源码：`cargo run --quiet -- 格式化`
- 类型检查（cargo check 与 tsc）：`cargo run --quiet -- 类型检查`
- 校验（产物注册、颜色格式、键覆盖、对比度和产品图标）：`cargo run --quiet -- 校验`
- 验证清单默认设置、命令和工作区能力：`cargo run --quiet -- 验证设置`
- 判断发布权限：`cargo run --quiet -- 发布策略 --引用 refs/tags/v<版本>`（CI 添加 `--输出 "$GITHUB_OUTPUT"`）
- 校验并上传已打包版本：`cargo run --quiet -- 发布上传 --引用 refs/tags/v<版本> --目标 Release`
  （`--预演` 只做本地校验；`--目标 市场` 仍受主线与令牌约束）
- 构建主题并同步 `package.json`：`cargo run --quiet -- 主题`（`--监视` 添加调试标记）
- 对照已安装的 VS Code 检查自定义 CSS：`cargo run --quiet -- 检查样式`
- 按需运行离线性能基准：`cargo run --quiet -- 性能基准`（不属于常规完整检查；
  独立工作台基准 `cargo run --quiet -- 工作台基准 --浏览器工具 <playwright 目录>`
  另见 `基准/README.md`）
- 打包 VSIX（无需 Node.js）：`cargo run --release --quiet -- 打包`
- 自动生成变更日志：`cargo run --quiet -- 变更日志`
- 预览当前版本发布说明：`cargo run --quiet -- 发布说明`
- 刷新随附的 VS Code 数据与键表：`cargo run --quiet -- 更新默认数据`
- 生成产品图标字体：`cargo run --quiet -- 生成自有字形`、`cargo run --quiet -- 生成导入字形`
- 发布：推送 `v*` 标签后由 GitHub Actions 自动构建并上传 VSIX
  （`.github/workflows/release.yml`，发布说明从 Git 提交范围生成；先校验标签与版本一致、产物与提交一致，
  再跑静态检查与单元测试）；非主线或预发布版本只创建候选 Release。正式标签提交
  已进入 `main` 且配置仓库 Secrets `VSCE_PAT`、`OVSX_PAT` 后，同一
  VSIX 还会发布到 VS Code 扩展市场与 Open VSX（未配置时自动跳过）

任何改动完成前都要跑 `cargo run --quiet -- 检查`。

## 提交

提交信息使用**简体中文类型前缀**（`新增:` / `修复:` / `文档:` / `测试:` / `重构:` /
`杂务:` / `初始化:`，格式为 `类型: 描述`），完整约定见
[CONTRIBUTING.md](CONTRIBUTING.md)。

每次创建或修订 Git 提交后，自动为本机安装该提交对应的新扩展，无需再次询问：

1. 从该提交创建临时、隔离的源码副本，避免将未提交的用户改动带入 VSIX，
   也不改动工作区文件。
2. 生成固定蓝色的主题：`cargo run --quiet -- 主题`，
   再执行 `cargo run --quiet -- 打包`；保留完整 Git 历史与标签供日志生成。
3. 读取该提交的 `package.json` 版本，使用本机 VS Code CLI 执行
   `code --install-extension /绝对路径/AdwCode-<版本>.vsix --force`，
   使相同版本号的开发提交也能覆盖安装；不先卸载现有扩展。
4. 核对安装结果并清理临时副本。CLI 不可用或安装失败时，明确报告未完成安装，
   不把“打包成功”当作“已安装”。

安装后不得执行窗口重载、加载器启用或重新加载命令。
如 VS Code 提示重载，由操作者自行决定；磁盘安装完成不代表当前窗口已运行新代码。
本机安装不等于推送或发布，不因此自动打版本标签或创建 Release。

## 中文接口

项目自有 API（含模块、函数与类型名称）、配置键、命令标识、脚本参数、文件名和目录名，
除下列特例外，必须采用简体中文，不保留英文兼容别名：

- 外部平台、协议、文件格式或工具规定的接口、字段和标准文件名，例如
  `activate`、`deactivate`、VS Code 颜色键、`package.json`、`README.md`、`AGENTS.md`。
- 第三方 API、命令、原始文件名及上游标识，保持与来源一致。
- 专有名称、许可证标识与许可证原文、文件扩展名，以及项目规定的 `adwcode` 前缀。

特例必须有外部规范、上游来源或本项目明确约定作为依据；习惯使用英文不是特例。
新增或重命名时同步更新调用、清单、文档与测试，不随意翻译外部强制标识。
升级迁移见 [中文接口迁移](文档/08-中文接口迁移.md)。

项目自有目录及子目录必须采用简体中文。`.git`、`.github`、`.vscode`、`LICENSES`
及工具生成的缓存、构建目录保留约定名称；第三方原始目录保留上游名称。
VSIX 内的 `extension/` 是标准归档根目录，仓库源码使用 `扩展/`。
目录更名须同步更新导入与路径引用、构建、打包、CI、测试和文档，不保留旧目录兼容入口。
