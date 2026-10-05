# 贡献指南

感谢参与 AdwCode。本项目让 VS Code 在 GNOME 下看起来像原生应用：颜色取自
libadwaita 的实测取值，语法高亮对齐 GtkSourceView 方案，几何按 GNOME 的实际参数
设计——三者都以实测数据为准，不凭感觉调色。

## 开发环境

| 依赖 | 用途 |
| --- | --- |
| Python 3.9+ | 生成主题、校验与打包（CI 与本地开发使用 3.14 自由线程版本） |
| [fontTools](https://github.com/fonttools/fonttools) | 仅重新生成导入的单色字体或备用自有字体时需要，见产品图标说明 |
| [skia-pathops](https://github.com/fonttools/skia-pathops) | 仅再生成资产时处理描边、裁切与字重内缩 |
| [nanoemoji](https://github.com/googlefonts/nanoemoji) | 仅重新生成 `product-icons/adwaita-icons.ttf` 时需要 |
| VS Code | 供 `python3.14t src/check_css.py` 对照已安装的构建检查 `extras/*.css` |

## 常用命令

先安装 [uv](https://docs.astral.sh/uv/getting-started/installation/) 与 Node.js，
再安装自由线程 Python、Meson、Ninja、ty 和 TypeScript 编译器：

```sh
uv python install 3.14t
python3.14t -m pip install meson ninja ty
npm install -g typescript
meson setup builddir
meson compile -C builddir themes    # 生成主题并同步清单
meson compile -C builddir lint      # 静态与类型检查
meson compile -C builddir check     # 主题校验
meson test -C builddir --print-errorlogs  # 完整检查及离线测试
meson compile -C builddir package   # 打包 VSIX
meson compile -C builddir --clean   # 清理 Meson 构建目录中的产物
```

`package.json` 中保留了等价别名，`npm run build`、`npm run check`、`npm test`、
`npm run package` 等与上面的目标一一对应。

## 改动流程

开始前阅读 [架构与实现状态](docs/03-架构与实现状态.md)，按
[开发、验证与发布](docs/04-开发验证与发布.md) 选择检查方式。
自动化测试应使用独立环境，避免重载正在使用的工作窗口；修改受监视文件前检查
自动重载设置。

- `themes/` 是生成产物，不要手工编辑；改完映射或调色板后重新运行 `meson compile -C builddir themes`，
  并让生成结果随提交一起入库。
- 新增颜色键必须存在于 `src/vscode_defaults/registry_keys.json`，或在
  `build.LEGACY_KEYS` 中，否则 `meson compile -C builddir check` 会失败。
- 主题 JSON 中的颜色必须是十六进制（`#rrggbb` / `#rrggbbaa`）：VS Code 会忽略
  CSS Color 4 写法（如 `rgb(0 0 6 / 36%)`），请使用 `palette.as_hex()`。
- `contrastBorder` / `contrastActiveBorder` 只属于高对比度主题；在普通主题中定义
  它们会到处多出描边。
- 扩展为无构建步骤、无依赖的纯 JavaScript，不要引入运行时依赖。

## 提交与 PR

- README、项目文档、代码注释、界面提示与自有脚本说明统一使用简体中文。
  API、配置键、路径、专有名称、第三方命令和许可证原文保留原文。
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

- 提交前必须运行 `meson test -C builddir --print-errorlogs`，静态、类型、主题和离线测试均应通过。
- PR 描述请填写仓库自带的模板，逐项确认约束检查。
- 发布：`package.json` 的 `version` 是版本号的唯一事实源；推送形如 `v1.0.0` 的
  标签后，GitHub Actions 会自动构建并上传 VSIX 到对应的 Release（发布说明由提交
  自动生成），并在仓库配置 Secrets `VSCE_PAT`、`OVSX_PAT` 后同步发布到
  VS Code 扩展市场与 Open VSX；未配置 Secrets 时只跳过市场发布。

## 许可证

本项目以 **AGPL-3.0-or-later** 发布（全文见 [LICENSE](LICENSE)）。提交贡献即表示
同意以该许可证发布你的自有代码贡献。引入第三方代码或组合依赖前，先确认
AGPL-3.0-or-later 兼容性；独立艺术资产保留原许可、署名和分发边界。
所有第三方来源均须在 [docs/01-第三方许可证.md](docs/01-第三方许可证.md) 登记。
