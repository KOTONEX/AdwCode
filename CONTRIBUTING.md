# 贡献指南

感谢参与 AdwCode。本项目让 VS Code 在 GNOME 下看起来像原生应用：颜色取自
libadwaita 的实测取值，语法高亮对齐 GtkSourceView 方案，几何参考 GNOME 的实际参数
设计；配色与布局调整以可核对的上游取值和界面验证为依据。

## 开发环境

| 依赖 | 用途 |
| --- | --- |
| 最新稳定版 Rust（`rust-toolchain.toml` 使用 `stable`） | 生成主题、校验、打包与基准 |
| 最新稳定版 Node.js 与 TypeScript 编译器 | 全部 TS 的严格编译、产物语法检查与离线测试 |
| VS Code | 供 `cargo run --quiet -- 检查样式` 对照已安装的构建检查 `附加外观/*.css` |

## 常用命令

先按 [rustup](https://rustup.rs/) 安装 Rust（`rust-toolchain.toml` 选择 `stable`，
包含 clippy/rustfmt），再安装最新稳定版 Node.js 与 TypeScript 编译器。
每次开始开发前更新工具链，具体规范见 [工具链政策](AGENTS.md#工具链政策)：

```sh
rustup update stable
npm install -g typescript@latest
cargo build

cargo run --quiet -- 主题          # 生成主题并同步清单
cargo run --quiet -- 检查          # 完整检查：fmt/clippy/test、校验、TS 编译与离线测试
cargo run --quiet -- 格式化        # cargo fmt
cargo run --quiet -- 校验          # 主题与产品图标校验
cargo run --quiet -- 打包          # 打包 VSIX
cargo run --quiet -- 变更日志      # 自动生成变更日志
cargo run --quiet -- 发布说明      # 预览当前版本发布说明
```

`package.json` 中保留了等价别名，`npm run 构建`、`npm run 校验`、`npm test`、
`npm run 打包` 等与上面的命令一一对应。

## 改动流程

开始前阅读 [架构与实现状态](文档/03-架构与实现状态.md)，按
[开发、验证与发布](文档/04-开发验证与发布.md) 选择检查方式。
自动化测试应使用独立环境，避免重载正在使用的工作窗口。扩展不监视文件，
也不会自动重载。

- `主题/` 是生成产物，不要手工编辑；改完映射或调色板后重新运行
  `cargo run --quiet -- 主题`，并让生成结果随提交一起入库。
- 新增颜色键必须存在于 `源码/VSCode默认数据/registry_keys.json`，或在
  `主题生成::补充颜色键` 中，否则 `cargo run --quiet -- 校验` 会失败。
- 主题 JSON 中的颜色必须是十六进制（`#rrggbb` / `#rrggbbaa`）：VS Code 会忽略
  CSS Color 4 写法（如 `rgb(0 0 6 / 36%)`），请使用 `调色板::规范颜色格式`。
- `contrastBorder` / `contrastActiveBorder` 只属于高对比度主题；在普通主题中定义
  它们会到处多出描边。
- 扩展、附加脚本、测试与基准均以 TypeScript 维护，不引入第三方运行时依赖。
  `cargo run --quiet -- 编译脚本` 输出到 `builddir/脚本/`；打包自动重新编译，源码与 JS 产物一起分发。

## 提交与 PR

- README、项目文档、代码注释、界面提示与自有脚本说明统一使用简体中文。
  项目自有 API（含模块、函数与类型名称）、配置键、命令标识、脚本参数、文件名和目录名，
  除明确特例外必须采用简体中文，不保留英文兼容别名。特例仅包括外部平台、协议、
  文件格式或工具规定的接口、字段与标准文件名，第三方 API、命令、原始文件名与
  上游标识，以及专有名称、许可证标识与原文、文件扩展名和项目规定的 `adwcode` 前缀。
  特例须有规范、来源或项目明确约定作为依据；习惯使用英文不构成例外。
  自有目录及子目录必须采用简体中文；工具规定的 `.git`、`.github`、`.vscode`、
  `LICENSES`、生成的缓存与构建目录、VSIX 标准 `extension/` 根目录，以及第三方原始目录
  保留约定名称。目录更名须同步更新导入与路径引用、构建、打包、CI、测试和文档。
  改名时同步更新调用、清单、文档与测试；详见 [中文接口迁移](文档/08-中文接口迁移.md)。
- 提交信息使用**简体中文类型前缀**（格式 `类型: 描述`，半角冒号 + 空格）：

  | 类型 | 用途 | 对应英文约定 |
  | --- | --- | --- |
  | `新增:` | 新功能 / 新行为 | feat |
  | `修复:` | 修 bug、修行为偏差 | fix |
  | `文档:` | 只改文档 | docs |
  | `测试:` | 只改测试 | test |
  | `重构:` | 不改变行为的重构 | refactor |
  | `杂务:` | 构建、依赖、CI、清理等 | chore |
  | `初始化:` | 仓库 / 模块的初始提交 | init |

- 提交前必须运行 `cargo run --quiet -- 检查`，格式、clippy、测试、校验与离线 JS 测试均应通过。
- 提交标题是变更日志的输入，请写清具体变化；无需编辑 `CHANGELOG.md`。
  主线版本标签划分发布范围，生成文件位于 `builddir/CHANGELOG.md`，不提交到仓库；
  完整 Git 历史与标签是生成所需输入。未分类的旧标题归入“其他”，合并提交不重复列出。
- PR 描述请填写仓库自带的模板，逐项确认约束检查。
- VSIX 产物不变的改动只提交与推送，不更新发布版本号、不打版本标签、不创建 Release。
  是否发布以实际打包内容的变化为依据，不因产生新提交而自动发布。
- 发布：`package.json` 的 `version` 是版本号的唯一事实源；推送与清单版本一致的 `v<版本>`
  标签后，GitHub Actions 会自动构建并上传 VSIX 到对应的 Release，变更日志及说明
  从版本范围内的 Git 提交标题生成；配置 Secrets `VSCE_PAT`、`OVSX_PAT` 后，同一 VSIX 会同步发布到
  VS Code 扩展市场与 Open VSX；未配置 Secrets 时只跳过市场发布。

## 许可证

本项目自有代码采用 `AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later`，自有资产采用
`AGPL-3.0-or-later OR CC-BY-SA-4.0+`。提交自有贡献即同意同时提供该类别的两种
可选许可，使用者可任选其一；范围与全文见 [许可声明](许可声明.md)。

第三方内容及其衍生部分保留原许可；引入前核对兼容性、署名和分发边界，在
[第三方许可证](文档/01-第三方许可证.md) 登记。包含第三方内容的文件不能因
其中的自有贡献提供双重许可而整体重新许可。
