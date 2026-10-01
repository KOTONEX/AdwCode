# 贡献指南

感谢参与 AdwCode。本项目让 VS Code 在 GNOME 下看起来像原生应用：颜色取自
libadwaita 的实测取值，语法高亮对齐 GtkSourceView 方案，几何按 GNOME 的实际参数
设计——三者都以实测数据为准，不凭感觉调色。

## 开发环境

| 依赖 | 用途 |
| --- | --- |
| Python 3.9+ | 生成主题、校验与打包（CI 与本地开发使用 3.14） |
| [nanoemoji](https://github.com/googlefonts/nanoemoji) | 仅重新生成 `product-icons/adwaita-icons.ttf` 时需要 |
| VS Code | 供 `python3 src/check_css.py` 对照已安装的构建检查 `extras/*.css` |

## 常用命令

```sh
make lint      # 静态检查（py_compile 全部 Python 源文件 / package.json / typecheck）
make typecheck # 类型检查：mypy（src、tests）+ tsc（extension.js，缺工具时跳过）
make check     # 校验已生成的主题（键覆盖、未知键、对比度、产品图标）
make test      # 离线单元测试（unittest，CI 可跑）
make build     # 生成主题并同步 package.json
make package   # 打包 AdwCode-<版本>.vsix
make clean     # 清理本地产物（__pycache__ 等）
```

`package.json` 中保留了等价别名，`npm run build`、`npm run check`、`npm test`、
`npm run package` 等与上面的目标一一对应。

## 改动流程

- `themes/` 是生成产物，不要手工编辑；改完映射或调色板后重新运行 `make build`，
  并让生成结果随提交一起入库。
- 新增颜色键必须存在于 `src/vscode_defaults/registry_keys.json`，或在
  `build.LEGACY_KEYS` 中，否则 `make check` 会失败。
- 主题 JSON 中的颜色必须是十六进制（`#rrggbb` / `#rrggbbaa`）：VS Code 会忽略
  CSS Color 4 写法（如 `rgb(0 0 6 / 36%)`），请使用 `palette.as_hex()`。
- `contrastBorder` / `contrastActiveBorder` 只属于高对比度主题；在普通主题中定义
  它们会到处多出描边。
- 扩展为无构建步骤、无依赖的纯 JavaScript，不要引入运行时依赖。

## 提交与 PR

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

- 提交前至少跑通 `make lint`、`make check` 与 `make test`。
- PR 描述请填写仓库自带的模板，逐项确认约束检查。
- 发布：`package.json` 的 `version` 是版本号的唯一事实源；推送形如 `v1.0.0` 的
  标签后，GitHub Actions 会自动构建并上传 VSIX 到对应的 Release（发布说明由提交
  自动生成），并在仓库配置 Secrets `VSCE_PAT`、`OVSX_PAT` 后同步发布到
  VS Code 扩展市场与 Open VSX；未配置 Secrets 时只跳过市场发布。

## 许可证

本项目以 **AGPL-3.0-or-later** 发布（全文见 [LICENSE](LICENSE)）。提交贡献即表示
同意以该许可证发布你的贡献；引入第三方代码、数据或资产前，先确认其许可证与
AGPL-3.0-or-later 兼容，并在 [docs/01-第三方许可证.md](docs/01-第三方许可证.md)
登记。
