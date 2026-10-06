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
  的行为；`测试/验证状态.cjs`、`测试/验证设置.cjs` 保持 JavaScript，
  由 `adwcode 检查` 调用 Node 执行。
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
