# AdwCode —— 跟随 GNOME 的 VS Code 主题

跟随 GNOME 桌面的 Visual Studio Code 主题：调色板直接取自 **libadwaita 1.10**
（GNOME 51），语法高亮对齐 **GNOME Builder** 的 GtkSourceView 方案，强调色可以
跟随 GNOME 系统设置。

本项目以 AGPL-3.0-or-later 发布；使用的第三方组件与数据登记在
[docs/01-第三方许可证.md](docs/01-第三方许可证.md)，许可证声明见文末
[「许可证」](#许可证)。

## 特性

- **libadwaita 1.10 配色** —— 窗口 `#222226` / 视图 `#1d1d20` / 标题栏与侧栏
  `#2e2e32` / 弹出层 `#36363a`；浅色模式的半透明前景
  （`rgb(0 0 6 / 80%)`）按各自表面合成；15% `currentColor` 边框；
  7% / 12% 的悬停与激活填充。
- **界面颜色覆盖** —— 对照随附的 VS Code 颜色注册表与内置主题校验颜色键，
  覆盖聊天、智能体、笔记本、行内编辑、测试、合并编辑器与内联提示等界面。
  具体覆盖情况可运行 `make check` 查看。
- **GNOME Builder 语法高亮** —— 由随附的 GtkSourceView `Adwaita` /
  `Adwaita-dark` 方案生成，并附带 `semanticTokenColors` 语义高亮。
- **强调色** —— 支持 GNOME 全部九种强调色（blue、teal、green、yellow、orange、
  red、pink、purple、slate）。默认构建 blue，可用 `--accents all` 生成其余颜色，
  或让扩展跟随 `org.gnome.desktop.interface accent-color`。
- **变体** —— 默认语法高亮（使用 VS Code 自带的语法颜色）与
  彩色状态栏变体，以及高对比度主题。
- **产品图标主题** —— Adwaita 风格的窗口控制按钮字形（在
  `window.controlsStyle` 为 `custom` 时生效）。
- **GNOME 外观（CSS）** —— 为整个工作台带来 Adwaita 几何：9px 的按钮/输入框/
  列表行/编辑标签页、15px 的弹出层与快速输入、6px 小控件、Adwaita 阴影、
  内缩细滚动条滑块。
- **推荐设置** —— 一条命令对齐 GNOME Builder 的布局（Adwaita Mono、关闭缩略图、
  关闭面包屑、紧凑标签高度等）。
- **终端配色** —— 由 GNOME 调色板推导的 16 色 ANSI。

## 主题

| 标签 | 说明 |
| --- | --- |
| `Adwaita 深色` / `Adwaita 浅色` | Builder 语法，标准状态栏 |
| `Adwaita 深色 · 彩色状态栏` / `Adwaita 浅色 · 彩色状态栏` | 状态栏填充强调色 |
| `Adwaita 深色 · 默认语法高亮` / `Adwaita 浅色 · 默认语法高亮` | 使用 VS Code 自带 token 颜色 |
| `Adwaita <强调色> 深色` / `Adwaita <强调色> 浅色` | 非蓝色强调色，按需生成 |
| `Adwaita 深色 高对比度` / `Adwaita 浅色 高对比度` | libadwaita 高对比度参数 |

产品图标主题 `Adwaita` 替换四个窗口控制字形（`chrome-close`、
`chrome-maximize`、`chrome-minimize`、`chrome-restore`）。它们只在
`window.controlsStyle` 设为 `custom` 时出现——默认的 `native` 由 GNOME 自己绘制
按钮，这是推荐配置。标题栏布局按钮、菜单栏溢出与导航箭头刻意保留系统 codicon：
Adwaita 实心符号字形在这些位置明显偏粗。

## 安装

从 [Releases](https://github.com/KOTONEX/AdwCode/releases) 下载由 CI 自动构建的
`AdwCode-<版本>.vsix`（把 `<版本>` 换成实际版本号，文件名以 Releases 页面上的附件为准），然后安装：

```sh
code --install-extension AdwCode-<版本>.vsix
```

也可以从源码构建（仓库中不包含 VSIX，构建产物会被 `.gitignore` 忽略；
`src/package.py` 会打印生成的文件名）：

```sh
python3 src/build.py            # blue + 当前系统强调色
python3 src/package.py          # 生成 AdwCode-<版本>.vsix
code --install-extension AdwCode-<版本>.vsix
```

## 设置

### 手动配置

```jsonc
"window.titleBarStyle": "custom",
"window.controlsStyle": "native",        // 由 GNOME 绘制窗口按钮
"window.autoDetectColorScheme": true,
"workbench.preferredDarkColorTheme": "Adwaita 深色",
"workbench.preferredLightColorTheme": "Adwaita 浅色",
"workbench.productIconTheme": "adwaita",
"editor.renderLineHighlight": "none",    // libadwaita 没有当前行边框
"workbench.tree.indent": 12,
"workbench.iconTheme": null,             // 没有 Adwaita 文件图标主题
"editor.fontFamily": "Adwaita Mono"      // GNOME 48+ 自带
```

GNOME 扩展
[Rounded Window Corners](https://github.com/flexagoon/rounded-window-corners)
（原版已停止维护，这里使用社区维护的 fork）可对应 `--window-radius: 15px`。

### GNOME 外观（CSS）

颜色无法改变几何，因此 `extras/gnome-look.css`（通过同一个 Custom CSS 加载器
生效）为工作台提供 Adwaita 形状：

- 定义 VS Code 1.140 引用却从未声明的设计令牌（`--vscode-cornerRadius-*`、
  `--vscode-spacing-*`、`--vscode-shadow-*`），一次性修正约 250 条规则
  （快速输入、建议列表、对话框、下拉框、通知、面板标题……）；
- 补充 AdwTabBar 风格的圆角标签页（9px）、内缩菜单项、圆角标题栏按钮、
  内缩圆角列表行、9px 按钮/输入框、标题栏胶囊搜索框以及内缩细滚动条滑块；
- 统一侧栏、面板、通知与编辑器工具栏按钮，以及设置行、查找选项、复选框和
  对话框外观；键盘焦点环在容易裁切的区域使用内侧描边；
- 默认使用 `window.menuBarVisibility: compact` 折叠菜单，汉堡按钮保持
  VS Code 原生的活动栏位置；把设置改回 `classic` 或 `visible` 即可恢复完整菜单栏。

执行 **Adwaita: 安装 GNOME 外观（CSS）**（只想改窗口按钮则用
**Adwaita: 生成仅关闭按钮的窗口控件 CSS**）。开发时可在设置中开启
`adwcode.autoReload`：改动 `extras/*.css`、`extras/*.js`、`themes/*.json` 或扩展代码后，
会自动重新应用 Custom CSS 并重载窗口。VS Code 升级后补丁会被
覆盖：重新执行加载器的 **Reload Custom CSS and JS**，或再跑一次命令。

`python3 src/check_css.py` 会校验 `extras/` 中每个类选择器在已安装的 VS Code 里
是否仍然存在（由 JavaScript 创建的类名会在 bundle 中搜索），以及样式引用的每个
`var(--vscode-*)` 是否都有定义——每次 VS Code 升级后都应运行。

### 一键应用设置

**Adwaita: 应用推荐设置** 会写入 GNOME Builder 风格的布局默认值
（Adwaita Mono、关闭缩略图、关闭面包屑、紧凑标签高度、12px 树缩进、平滑滚动、
关闭行号边栏差异装饰等）。被覆盖的旧值会被记住，
**Adwaita: 恢复推荐设置** 可逐项恢复。

推荐设置还会开启 `adwcode.autoReload`。文件变化会触发整个窗口重载，可能中断
扩展会话或调试任务。需要连续工作时，可关闭此项并手动应用外观更新。

### 外观安装状态

执行 **Adwaita: 查看外观安装状态**，可检查加载器是否安装、每个外观文件的
副本是否与源码一致、是否加入加载器配置，以及磁盘上的工作台 HTML 是否注入
当前版本。面板支持手动刷新，不会修改配置或重载窗口。

磁盘补丁已更新不代表当前窗口已加载新样式。工作结束后再手动重载窗口，
避免中断当前工作。

### 仅保留关闭按钮

GNOME 默认布局只显示关闭按钮（`':close'`）：

```sh
gsettings get org.gnome.desktop.wm.preferences button-layout
```

- `"window.controlsStyle": "native"`（推荐）由 GTK 按该布局绘制，无需额外配置。
- VS Code 的自绘控件始终绘制最小化、最大化/还原与关闭三个按钮（固定 46px 宽、
  容器 138px），颜色主题与产品图标主题都无法隐藏。若要在自绘控件上保持 GNOME
  布局，执行 **Adwaita: 生成仅关闭按钮的窗口控件 CSS**：它会写入
  `~/.config/adwcode/controls-close-only.css`，并在检测到
  [Custom CSS and JS Loader](https://marketplace.visualstudio.com/items?itemName=be5invis.vscode-custom-css)
  时自动加入 `vscode_custom_css.imports`；否则把设置片段复制到剪贴板。先执行一次
  **Enable Custom CSS and JS**（该扩展需要 VS Code 安装目录的写权限）再重载窗口。

## 强调色

```sh
python3 src/build.py --accents all          # 全部九种强调色（18+ 个主题）
python3 src/build.py --accents blue,teal    # 指定子集
python3 src/build.py --no-system            # 仅 blue
```

随附扩展会监听 GNOME 强调色偏好并切换所选 Adwaita 主题到对应变体
（`adwcode.autoAccent`，默认 `true`；只会改动以 `Adwaita` 开头的主题）。
也可通过命令 **Adwaita: 立即同步强调色** 手动应用。
扩展为无构建步骤、无依赖的纯 JavaScript。

## 目录结构

```text
AdwCode/
├── AGENTS.md                    自动化代理（含 AI）说明
├── CHANGELOG.md                 更新日志
├── CONTRIBUTING.md              贡献指南
├── LICENSE                      AGPL-3.0 全文
├── Makefile                     统一命令入口
├── README.md                    本文件
├── package.json                 扩展清单，版本号唯一事实源
├── src/                         构建管线与可导入模块
│   ├── build.py                 生成主题并同步 package.json
│   ├── check_css.py             对照已安装的 VS Code 检查 extras/*.css
│   ├── mapping.py               VS Code 颜色键到 Adwaita 角色的映射
│   ├── package.py               打包 VSIX
│   ├── palette.py               libadwaita 颜色角色与合成工具
│   ├── tokens.py                GtkSourceView 样式名到 TextMate 作用域的映射
│   ├── update_defaults.py       刷新 VS Code 默认主题数据与键表
│   ├── gtksourceview_xml/       随附的 GtkSourceView 方案（LGPL-2.1+）
│   └── vscode_defaults/         解析后的 VS Code 默认 token 颜色（MIT）与键表
├── tests/                       离线单元测试
│   └── test_adwcode.py          颜色运算、调色板、语法、主题、CSS
├── themes/                      生成的主题 JSON（自动生成，请勿手工编辑；已提交）
├── product-icons/               产品图标主题（Adwaita 符号字形与字体）
├── extras/                      通过 Custom CSS 加载的样式表
├── extension/                   强调色同步扩展
├── docs/                        深入文档与第三方许可证登记
└── .github/                     CI、发布工作流与议题 / PR 模板
```

## 环境要求

| 依赖 | 要求 | 用途 |
| --- | --- | --- |
| VS Code | ≥ 1.100（`package.json` 的 `engines`） | 运行主题与扩展 |
| Python | 3.9+（CI 与本地开发使用 3.14） | 仅构建与打包需要 |
| GNOME | 提供 `org.gnome.desktop.interface accent-color`（GNOME 47 起） | 扩展跟随系统强调色；没有该键时手动选择主题变体 |
| VS Code 安装目录写权限 | — | Custom CSS and JS Loader 注入自定义 CSS 的要求 |

## 故障排查

### VS Code 升级后 CSS 失效

升级会覆盖注入的自定义 CSS：重新执行 **Adwaita: 安装 GNOME 外观（CSS）**，
或让加载器执行 **Reload Custom CSS and JS**，随后运行
`python3 src/check_css.py` 确认选择器与设计令牌仍然有效。

### 强调色不同步

检查 `adwcode.autoAccent` 是否为 `true`（只对以 `Adwaita` 开头的当前主题生效），
或执行命令 **Adwaita: 立即同步强调色**。

### 产品图标按钮不出现

产品图标主题只在 `window.controlsStyle` 为 `custom` 时生效；默认的 `native`
由 GNOME 绘制窗口按钮（推荐配置），此时不会使用 Adwaita 字形。

## 开发

项目说明按用途拆分：

- [自动化开发说明](AGENTS.md)：项目约定与智能体工作入口。
- [架构与实现状态](docs/03-架构与实现状态.md)：源数据、外观安装链路与功能边界。
- [开发、验证与发布](docs/04-开发验证与发布.md)：检查、手动预览和发布步骤。
- [贡献规范](CONTRIBUTING.md)：提交格式与贡献要求。

需要 Python 3.9+（CI 与本地开发使用 3.14）：

```sh
python3 src/build.py --check             # 键覆盖、未知键、对比度
python3 src/check_css.py                 # 自定义 CSS 与已安装 VS Code 的比对
python3 src/build.py --watch             # 给主题加 _watch，编辑 JSON 即时生效
python3 src/update_defaults.py           # 刷新 VS Code 默认主题数据与键表
python3 -m unittest discover -s tests -p 'test_*.py'   # 颜色运算、调色板、语法、主题、CSS
```

标准命令入口是 `make lint`、`make typecheck`、`make check`、`make test`、`make build`、
`make package` 与 `make clean`，语义见 Makefile。贡献流程、提交规范与自测要求
见 [CONTRIBUTING.md](CONTRIBUTING.md)。

`--check` 会把生成的主题与官方颜色注册表、内置主题比对：未知键（拼写错误）会
导致检查失败，缺失键会被列出，并校验明暗两种模式的对比度。

图标字体由 `product-icons/scalable/` 中的 SVG 经
[nanoemoji](https://github.com/googlefonts/nanoemoji) 生成：

```sh
nanoemoji --color_format glyf_colr_1 --family adwaita-icons \
  --output_file product-icons/adwaita-icons.ttf product-icons/scalable/*.svg
```

## 许可证

本项目以 **AGPL-3.0-or-later** 发布，全文见 [LICENSE](LICENSE)；与 GNOME 基金会
无隶属关系。随附的第三方组件与数据（GtkSourceView 样式方案、VS Code 默认主题
数据、piousdeer/vscode-adwaita 的 TextMate 作用域映射与产品图标字形等）登记在
[docs/01-第三方许可证.md](docs/01-第三方许可证.md)。
