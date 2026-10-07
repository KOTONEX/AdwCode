# Rust 迁移（`rust` 分支）

本分支把构建期 Python 代码迁移到 Rust，并以 cargo 与统一 CLI 取代 Meson。
运行时 JavaScript（`扩展/扩展.js`、`附加外观/窗口状态.js`）与 CSS 不在迁移范围：
VS Code 扩展宿主只运行 JavaScript，工作台补丁脚本运行在页面 DOM 中。

## 目录与命名

- 工作区根使用 `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`；它们是工具约定
  文件名，保留 ASCII，`target/` 为生成目录，不入库。
- Rust 源码位于 `源码/库/` 与 `源码/程序/`，模块文件名与其他项目目录使用简体中文；
  非 ASCII 模块名通过 `#[path]` 显式声明。
- 包名与二进制名固定为 `adwcode`（Cargo 强制 ASCII）；子命令、参数、输出与
  函数、类型、模块标识沿用简体中文约定。

## 命令入口

`adwcode <子命令>` 取代 `meson compile -C builddir <目标>` 与 npm 别名：

| 子命令 | 对应现状 |
| --- | --- |
| `主题` | `生成主题.py`（含 `--监视`） |
| `校验` | `生成主题.py --校验` |
| `检查样式` | `检查样式.py` |
| `打包` | `打包扩展.py` |
| `变更日志` | `生成变更日志.py 变更日志` |
| `发布说明` | `生成变更日志.py 发布说明 --标签` |
| `更新默认数据` | `更新默认数据.py` |
| `性能基准` | `运行基准.py` 与 `工作台基准.py` |
| `生成自有字形`、`生成导入字形` | `产品图标/` 两个生成器 |
| `检查` | `开发检查.py 静态检查`、`类型检查` 与离线测试的聚合 |
| `格式化` | `开发检查.py 格式化`（cargo fmt） |

## 依赖（拟）

运行普通工具只用 `serde_json`（`preserve_order`）、`zip`、`flate2`、`sha2`，
按需加入 `regex` 与 `ureq`；图标管线使用 `kurbo`、SVG 路径解析与
`write-fonts`/fontations。新增依赖必须为宽松许可并登记到
[第三方许可证](01-第三方许可证.md)。

## 产物一致性

- `主题/*.json` 与同步后的 `package.json`：字节一致；以金样例测试和
  `adwcode 主题` 后的 `git diff --exit-code -- 主题 package.json` 锁定。
- VSIX：归档条目与清单语义一致，ZIP 字节不作要求。
- 变更日志与发布说明：文本字节一致，使用现有测试夹具核对。
- 图标字体与 `渲染图标/*.svg`：允许重新生成；按 `产品图标/README.md` 的
  流程核对轮廓、字重与陈列文档，并更新受影响的缩略图。

## 测试与 CI

- `cargo test` 承接 `测试/test_主题.py`、`test_变更日志.py`、`test_发布流程.py`
  的行为；`测试/验证状态.cjs` 保持 JavaScript，由 `adwcode 检查` 调用 Node 执行。
  4.1.0 将清单设置验证和发布策略及其真实 Git 回归测试迁入 Rust，见下文。
- 阶段⑥改造 CI：安装 Rust（读取 `rust-toolchain.toml`）、缓存 cargo、
  `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`、
  生成主题并 diff、打包与发布；移除 Python、ruff、ty 与 Meson 步骤。

## 阶段

1. 迁移方案（本文档）与工作区骨架。
2. 统一 CLI 与金样例测试框架。
3. `源码/` 全部工具与测试迁移，主题输出保持一致。
4. 图标字体管线迁移，重新生成字体与预览。
5. Python 基准与测试运行器迁移，保留 JS 测试调用。
6. 以 cargo + CLI 取代 Meson，改造 CI、npm 脚本与文档，删除 Python 配置。
7. 全量验证与产物核对。

## 风险

- 浮点与 JSON 序列化差异：主题取值均为字符串或整数，风险低，由金样例锁定。
- 图标几何：Skia 布尔与描边、cu2qu 取整差异会改变轮廓，需要人工核对。
- 迁移期间 Python 与 Rust 并存，Meson 目标仍指向 Python，阶段⑥统一切换。


## 模块映射

| 旧 Python | Rust 模块 |
| --- | --- |
| `源码/调色板.py` | `源码/库/调色板.rs` |
| `源码/界面映射.py` | `源码/库/界面映射.rs` |
| `源码/语法映射.py` | `源码/库/语法映射.rs` |
| `源码/生成主题.py` | `源码/库/主题生成.rs` |
| `源码/打包扩展.py` | `源码/库/打包.rs` |
| `源码/生成变更日志.py` | `源码/库/变更日志.rs` |
| `源码/检查样式.py` | `源码/库/检查样式.rs` |
| `源码/更新默认数据.py` | `源码/库/更新默认数据.rs` |
| `源码/开发检查.py` | `源码/库/检查.rs`（cargo fmt/clippy/test + tsc/Node 与离线 JS 测试） |
| `基准/运行基准.py` | `源码/库/性能基准.rs` |
| `基准/工作台基准.py` | `源码/库/工作台基准.rs`（测量仍在 `基准/工作台基准.cjs`） |
| `产品图标/生成自有字形.py`、`生成导入字形.py` | `源码/库/图标生成.rs` |

## 实施记录

### 4.1.0 后续迁移

| 移除的 JavaScript 工具 | Rust 替代 |
| --- | --- |
| `测试/发布策略.cjs` | `源码/库/发布策略.rs`，入口 `adwcode 发布策略 --引用 refs/...` |
| `测试/验证发布.cjs` | 发布策略模块内的 Cargo 测试，使用真实临时 Git 仓库 |
| `测试/验证设置.cjs` | `源码/库/验证设置.rs`，入口 `adwcode 验证设置`，纳入完整检查 |

发布策略复用现有 SemVer 标签校验，拒绝非法版本与标签不匹配；缺少主线、
不完整历史或 Git 错误时失败。仅标签提交已进入 main 且无预发布标识时允许
市场发布；手动分支运行不授予市场发布权限，构建元数据中的连字符不算预发布。
使用 `--输出 "$GITHUB_OUTPUT"` 追加版本、候选和市场三个输出，发布工作流直接
复用 Rust 输出的版本号，不再通过 Node 读取清单。

清单验证保留原有默认主题、产品图标、字体、菜单、五个 Linux 命令及工作区能力
约束，并覆盖错误类型、缺失字段、重复命令等失败路径。完整检查读取当前磁盘的
清单；Cargo 测试包含合法清单与逐项破坏约束的回归。
主题、字体、CSS 和扩展运行逻辑没有变化；完整检查仍需要 Node.js 与 tsc，
用于执行真实 JS 的语法、类型和模拟宿主回归，工作台测量仍使用 Playwright。

### 首次迁移记录

- 非 ASCII 模块名必须用 `#[path]` 声明；标识符用纯中文或小写 ASCII，
  中文中间夹大写英文词会触发 Rust 的 `non_snake_case`。
- JSON 输出用 `serde_json`（`preserve_order`）与两空格缩进、结尾换行，
  与 Python `json.dumps(indent=2, ensure_ascii=False) + "\n"` 对齐；
  `主题/*.json` 与同步后的 `package.json` 由金样例测试逐字节锁定。
- 变更日志与发布说明除生成器注释改为 `adwcode 变更日志` 外与 Python 输出逐字节一致。
- VSIX 归档条目、顺序与内容与 Python 版本一致（ZIP 字节不作要求）。
- `registry_keys.json` 历史遗留的末尾换行已补齐，与生成器行为一致。
- `检查样式` 仍需要本机 VS Code 构建；`检查` 聚合不包含需要 VS Code 的项。
- 阶段⑥已删除 Python 源、Meson、Ruff/ty 配置与旧检查工具；`builddir/` 仅作为
  变更日志、发布说明与基准结果输出目录。
- 图标管线迁移后重新生成了 4 个 TTF 与 `渲染图标/` 轮廓：字形数量、映射与许可
  分组与 Python 版一致，轮廓数据因几何实现差异存在少量取整差异。
- 迁移后实跑基准：主题构建约 36 ms（Python 测得更慢），`检查样式` 约 2.2 s
  （比 Python 版慢，主要耗时在整份工作台 bundle 的正则扫描，可作为后续优化项）。
- 工作台基准的隔离配置显式关闭 `window.autoDetectColorScheme`：3.3.0 的
  `configurationDefaults` 会开启主题自动检测，否则基准无法用 `workbench.colorTheme`
  切换主题。
