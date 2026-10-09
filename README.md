<p align="center">
  <img src="资产/adwcode.png" alt="AdwCode 编辑器图标" width="128" height="128">
</p>

# AdwCode —— 跟随 GNOME 的 VS Code 主题

颜色取自 libadwaita 1.10（GNOME 51），语法高亮对齐 GNOME Builder 的
GtkSourceView 方案，界面采用固定蓝色强调色。可选 CSS 提供 GNOME 风格的圆角、
阴影、滚动条与窗口状态外观。

## 主题与图标

| 主题 | 说明 |
| --- | --- |
| `AdwCode 深色` / `AdwCode 浅色` | Builder 语法，标准状态栏 |
| `AdwCode 深色 · 彩色状态栏` / `AdwCode 浅色 · 彩色状态栏` | 状态栏填充强调色 |
| `AdwCode 深色 高对比度` / `AdwCode 浅色 高对比度` | libadwaita 高对比度参数 |

颜色主题覆盖界面、TextMate、语义高亮和 16 色终端。产品图标主题 `AdwCode`
映射 79 个标识，未覆盖的标识沿用 Codicons；本项目不提供文件图标主题。
来源与再生成方式见 [产品图标说明](产品图标/README.md)，逐项预览见
[产品图标陈列](文档/参考/产品图标陈列.md)。

## 安装

从 [GitHub Releases](https://github.com/KOTONEX/AdwCode/releases) 下载
`AdwCode-<版本>.vsix`，将 `<版本>` 替换为实际附件版本号：

```sh
code --install-extension "AdwCode-<版本>.vsix"
```

直接安装新版即可升级，无需卸载旧版。安装已打包的扩展不需要 Rust、Node.js 或
TypeScript 编译器，扩展无额外第三方运行时库依赖。旧设置、快捷键及接口更名见
[升级与旧接口](文档/使用/升级与旧接口.md)。

VS Code 最低版本以 [package.json](package.json) 的 `engines.vscode` 为准。
颜色与产品图标主题可独立使用；GNOME 外观命令只在 Linux 的本地 UI 宿主注册，
虚拟工作区与受限模式不可用。

## 外观设置

安装后默认跟随系统明暗与高对比度模式，采用 AdwCode 产品图标、GNOME 代码字体
与 Builder 风格布局。用户或工作区的显式设置优先，扩展不会改写这些设置。

可选的 GNOME 外观需要
[Custom CSS and JS Loader](https://marketplace.visualstudio.com/items?itemName=be5invis.vscode-custom-css)：
执行 **AdwCode: 安装 GNOME 外观（CSS）**，或用 **AdwCode: 选择外观组件** 多选组件，
再由操作者启用或更新加载器并手动重载窗口。加载器注入需要 VS Code 安装目录的写权限。

**AdwCode: 查看外观安装状态** 提供只读检查；**AdwCode: 移除自有外观加载引用**
保留其他用户加载项及磁盘副本。扩展不监视文件，不自动操作加载器或重载窗口。
详细设置、字体、窗口按钮和故障排查见 [外观与设置](文档/使用/外观与设置.md)。

## 从源码开发与构建

需要完整 Git 历史与标签，以及最新稳定版 Rust、Node.js 和 TypeScript。
先按 [工具链政策](AGENTS.md#工具链政策) 准备环境，再运行：

```sh
cargo run --quiet -- 主题
cargo run --quiet -- 检查
cargo run --release --quiet -- 打包
```

生成的 VSIX 位于仓库根目录，构建工具会打印文件名。Rust 开发工具位于 `源码/`；
扩展、附加脚本、测试与基准以 TypeScript 维护，编译到 `builddir/脚本/`。
打包重新编译并附带源码、许可证及运行产物。

- [文档目录](文档/README.md)：按使用、开发、参考与历史查阅。
- [贡献指南](CONTRIBUTING.md)：提交格式与贡献要求。
- [自动化开发规范](AGENTS.md)：中文接口、工具链、隔离验收及授权边界。
- [架构](文档/开发/架构.md)、[开发验证](文档/开发/开发验证.md)、[发布](文档/开发/发布.md)。
- [路线图](文档/开发/路线图.md)：尚未实施的候选能力。

## 许可证

自有代码采用 **AGPL-3.0-or-later 或木兰公共许可证第 2 版或其后续版本**，自有资产采用
**AGPL-3.0-or-later 或 CC BY-SA 4.0 或其后续版本**，使用者可任选其一。
范围与全文见 [许可声明](许可声明.md)；第三方及其衍生内容保留原许可，登记在
[第三方许可证](文档/参考/第三方许可证.md)。本项目与 GNOME 基金会无隶属关系。
