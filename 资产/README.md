# 扩展图标

AdwCode 使用原创的 GNOME 风格编辑器图标：暖白色圆角窗口、纯蓝色文件页签、
窄文件侧栏和带缩进的语法色块。通过布局与配色表达代码编辑器主题功能，采用
上方光照和浅立体侧面，背景透明。

## 文件与展示

- `adwcode.png`：1254 × 1254 透明 PNG，是最终图标的位图原件。
- `package.json` 的 `icon` 指向此 PNG，README 开头以 128 × 128 显示同一文件。
- 打包器将 PNG 纳入 VSIX，并登记扩展列表与详情页使用的图标资源。
- [设计说明](设计说明.md)：构图、制作方式与验证要求。
- [来源记录](来源.json)：许可、制作方式、尺寸与文件哈希。

本图标没有对应的 SVG 源文件；日常构建和安装直接读取随附 PNG，不需要图像
生成工具。重新设计时保留编辑器的识别结构，完成后同步来源记录和设计说明。

## 许可

图标由本项目按文字描述生成并编辑自有候选图像，未引入第三方徽标或图像素材。
项目对有权许可的图标内容采用 **CC BY-SA 4.0（或其后续版本）与 AGPL-3.0-or-later 双重许可**，
使用者可任选其一，并遵守所选许可的全部条款。SPDX 表达式为
`CC-BY-SA-4.0+ OR AGPL-3.0-or-later`，署名使用 AdwCode contributors。
声明见 [图标许可证](LICENSE)，两种许可全文分别随附于
[CC-BY-SA-4.0.txt](CC-BY-SA-4.0.txt) 与 [项目许可证](../LICENSE)。
此许可声明不授予任何第三方商标权利。

## 验证

```sh
meson test -C builddir --print-errorlogs
meson compile -C builddir 打包
```

检查透明通道、浅色与深色背景，以及 128、64、32、16 像素下的辨识度；核对
VSIX 中的 PNG 字节与仓库文件一致。打包与静态检查不代表当前 VS Code 窗口的
扩展列表已刷新，不通过窗口重载进行自动化验证。

设计原则参考 [GNOME 应用图标指南](https://developer.gnome.org/hig/guidelines/app-icons.html)；
清单字段见 [VS Code 扩展清单](https://code.visualstudio.com/api/references/extension-manifest)。
