# AGENTS.md

给自动化代理（含 AI）的项目说明；人类贡献者请看 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 新会话入口

1. 先运行 `git status --short`，区分已有改动、暂存内容与未跟踪文件；不要覆盖或
   顺手提交与任务无关的内容。版本读取 `package.json`，提交状态读取 Git。
2. 阅读本文件，按 [文档目录](文档/README.md) 选择当前说明：
   [架构](文档/开发/架构.md)、[开发验证](文档/开发/开发验证.md)、[发布](文档/开发/发布.md)。
   [路线图](文档/开发/路线图.md) 仅记录候选；`文档/历史/` 是阶段记录，不能当成当前操作说明。
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

需要最新稳定版 Rust（`rust-toolchain.toml` 使用 `stable`，含 clippy 与 rustfmt）、Node.js 和 TypeScript；脚本编译、产物语法检查和离线测试使用同一工具链政策。

## 工具链政策

- 本地开发、检查、打包及 CI 始终使用上游最新稳定版工具链，不固定 Rust、
  Node.js 或 TypeScript 的数字版本；禁止以 beta、RC、nightly、dev 版本代替稳定版。
  最新稳定版包括 Node.js 的最新正式发行版，不限于 LTS。
- Rust 通过 `rust-toolchain.toml` 的 `channel = "stable"` 选择；每次开始开发前运行
  `rustup update stable`。已有 stable 安装不会仅因执行 cargo 自动更新，不能把
  渠道配置视为已完成更新。不声明旧编译器的支持范围或添加固定版本门槛。
- Node.js 通过所用版本管理器或包管理器更新到最新正式版；CI 使用
  `node-version: "latest"` 与 `check-latest: true`。
  TypeScript 使用 npm 的稳定标签 `typescript@latest`，不使用 `next`、beta 或 RC。
  本地用 npm 安装时运行 `npm install -g typescript@latest`；由其他包管理器管理时，
  通过该管理器更新，并核对其版本与 npm 的 `latest` 一致。
- 开发前核对 `rustc --version`、`cargo --version`、`node --version`、`tsc --version`；
  必要时用 `npm view typescript@latest version` 核对稳定编译器版本。常规编译与打包
  不访问版本服务或擅自升级全局环境，工具链更新在准备阶段完成。
- 最新稳定版引入不兼容时，应修复代码、类型声明或配置并运行完整检查，不通过
  回退或重新固定旧工具链绕过问题。验收与基准记录当次实际版本；同一源码、
  同一工具链下验证打包确定性，不承诺不同编译器产物逐字节一致。
- `Cargo.lock` 和第三方库版本管理继续用于依赖解析；`edition = "2024"` 与 TS 的
  `target`、模块解析配置属于语言及运行兼容性约定，不是工具链版本锁定。
  Cargo 要求 Edition 显式使用合法值，不能写 `latest`；上游发布新 Edition 后，
  以最新稳定编译器验证迁移再更新，不省略字段退回旧 Edition。
- 官方渠道说明见 [Rust stable 与更新](https://rust-lang.github.io/rustup/basics.html)、
  [Node.js latest](https://github.com/actions/setup-node#supported-version-syntax) 和
  [TypeScript 安装](https://www.typescriptlang.org/download/)。

## 目录

目录与模块职责统一见 [架构](文档/开发/架构.md#目录与语言边界)。
图标调整查 [产品图标维护](产品图标/README.md) 与 [字形陈列](文档/参考/产品图标陈列.md)，
第三方引入查 [许可证登记](文档/参考/第三方许可证.md)，基准查 [运行说明](基准/README.md)。

## 约定

- 主题 JSON 中的颜色必须是十六进制（`#rrggbb` / `#rrggbbaa`）：VS Code 会忽略
  CSS Color 4 写法（如 `rgb(0 0 6 / 36%)`）。请使用 `调色板::规范颜色格式`。
- `contrastBorder` / `contrastActiveBorder` 只属于高对比度主题；在普通主题中定义
  它们会到处多出描边（VS Code 的 CSS 以 `unset` / `transparent` 作为回退）。
- 新增颜色键必须存在于 `源码/VSCode默认数据/registry_keys.json`，或在
  `主题生成::补充颜色键` 中，否则 `校验` 会失败。
- 扩展、附加脚本、测试与基准使用 TypeScript；模块工厂见 `扩展/外观服务.ts`。
  `cargo run --quiet -- 编译脚本` 严格编译到 `builddir/脚本/`，打包会重新编译并收录
  源码和运行产物。扩展不引入第三方运行时库；`类型声明/` 保持手写最小类型面，不引入 `@types` 依赖。
- `主题/` 是生成产物，不要手工编辑。
- `附加外观/*.css` 和编译后的 `builddir/脚本/附加外观/窗口状态.js` 通过「Custom CSS and JS Loader」扩展生效。
  非活动状态由脚本同步到导航容器，不恢复工作台祖先上的 `:has()` 规则；
  安装命令把已知源码与副本引用统一为一份副本，保留其他用户加载项。
  VS Code 1.140 在约 250 处引用 `--vscode-cornerRadius-*` / `--vscode-spacing-*`
  却从未定义它们，因此 `GNOME外观.css` 自行定义这些令牌以提供 Adwaita 几何。
  VS Code 升级后请
  运行 `cargo run --quiet -- 检查样式`：它会校验每个类选择器仍存在于已安装的构建中（由
  JavaScript 创建的类名会在 bundle 中搜索；项目自有状态类核对脚本的显式创建操作）。
- VSIX 产物不变的改动只提交与推送，不更新发布版本号、不打版本标签、不创建 Release。
  是否发布以实际打包内容的变化为依据，不因产生新提交而自动发布。
- 默认开发分支为 `主线`，远端跟踪引用为 `origin/主线`；CI 和发布策略使用该名称。
- 版本号唯一事实源是 `package.json` 的 `version`，发布流程见
  `.github/workflows/release.yml`。
- 不手工维护 `CHANGELOG.md`；提交标题与主线上的版本标签是日志输入。
  `cargo run --quiet -- 变更日志` 生成 `builddir/CHANGELOG.md`，打包时自动写入
  VSIX。完整历史和标签不可缺失，CI 必须完整检出；未提交修改不会进入日志。
- 不以“类名存在”或“磁盘补丁已更新”宣称当前窗口外观已验证。

## 常用命令

命令表和参数入口见 [开发验证](文档/开发/开发验证.md#常用命令)，发布命令见
[发布](文档/开发/发布.md)。统一 CLI 为 `adwcode`：开发期使用
`cargo run --quiet -- <子命令>`，release 构建后用 `./target/release/adwcode <子命令>`。

任何改动完成前都要跑 `cargo run --quiet -- 检查`。主题、字体等生成器修改后，
同步生成产物并核对差异；涉及 CSS 时另跑 `检查样式`。真实宿主和工作台采用独立环境，
不执行用户窗口重载。命令缺少必要环境时报告未完成，不将跳过当成验证成功。

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
用户升级见 [升级与旧接口](文档/使用/升级与旧接口.md)。

项目自有目录及子目录必须采用简体中文。`.git`、`.github`、`.vscode`、`LICENSES`
及工具生成的缓存、构建目录保留约定名称；第三方原始目录保留上游名称。
VSIX 内的 `extension/` 是标准归档根目录，仓库源码使用 `扩展/`。
目录更名须同步更新导入与路径引用、构建、打包、CI、测试和文档，不保留旧目录兼容入口。
