# 产品图标主题

`adwcode.json` 将随附字体中的单色字形映射到 VS Code 产品图标。
当前覆盖 **79 个图标标识**，使用 **68 个不同字形、3 个字体**。
语义相同的操作可复用字形，例如运行与继续、包与包符号。
文件类型图标由单独的文件图标主题负责，不受这里的映射影响。

全部字形的缩略图、用途、产品图标标识与原始 SVG 见
[产品图标陈列](../docs/06-产品图标陈列.md)。

## 来源与范围

选材优先级为 **Adwaita → GNOME Builder → MoreWaita**：先找语义相符的官方
字形，再由 Builder 补充开发专用符号，前两者缺项时才使用 MoreWaita。
该顺序用于确定映射，不是运行时字体回退；实际来源由每个标识的 `fontId` 确定。

| 来源 | 字形数 | 用途 | 字体及许可证 |
| --- | --- | --- | --- |
| Adwaita Icon Theme | 37 | 通用操作、窗口、终端、扩展、包与项目目录 | `adwaita-symbols.ttf`，选择 LGPL-3.0-only 授权 |
| GNOME Builder | 25 | 调试、版本控制、测试入口、补全与语言符号 | `builder-symbols.ttf`，CC-BY-SA-3.0 |
| piousdeer/vscode-adwaita | 6 | 侧栏与底部面板的成对布局状态 | `adwaita-icons.ttf`，GPL-3.0-only |
| MoreWaita | 0（另有 3 个备用字形） | 当前用途均有 Adwaita 对应字形，未启用 | `morewaita-symbols.ttf`，GPL-3.0-only，仅备用 |

布局状态对继续使用已验证的 Adwaita 派生字形，保留显示与隐藏的区别；没有
语义和状态都对应的替代对时，不将它们合并成同一个通用图标。

- 布局字形复用自
  [piousdeer/vscode-adwaita](https://github.com/piousdeer/vscode-adwaita)，SVG 位于
  `scalable/`；其字形派生自 GNOME 符号图标。
- Builder 使用
  [图标目录的 CC BY-SA 3.0 声明](https://github.com/GNOME/gnome-builder/blob/3bcd6fbd31c82b3ea6f540679d3a1d6b2e999fe0/data/icons/COPYING)，
  不能用项目代码的 GPL 声明替代图标许可。字体保留该许可并独立分发，不与
  GPL 或本项目自有字形合并。
- MoreWaita 使用
  [GPL v3](https://github.com/somepaulo/MoreWaita/blob/73e900822829768560f88084eababc03f664bc35/LICENSE)，
  作者与贡献者说明随附在 `imported/morewaita/AUTHORS`。
- Adwaita 官方图标有
  [LGPL v3 或 CC BY-SA 3.0 双重许可](https://github.com/GNOME/adwaita-icon-theme/blob/82d305723107d2ab619c0a4e4ec3136558bb4fe7/COPYING)，
  本项目选择 LGPL 分支；署名为 GNOME Project。

三个来源的完整提交号、上游路径、原文件 SHA-256、码点与产品图标映射记录在
[sources.json](sources.json)。`imported/` 中保存未经修改的原始 SVG、来源许可与
贡献者说明；颜色统一、坐标变换、曲线转换与字体封装由生成脚本完成。
`source_priority` 记录选材顺序，`enabled: false` 标记备用字体，不参与运行时加载。
字体移除画布外的导出残留，将官方描边展开为轮廓；可见复杂特效仍会导致生成失败。

## 字重与界面协调

当前字体在生成阶段减轻轮廓线条，字符占位宽度、基线、图标颜色及点击区域保持原值。
Adwaita 通常将轮廓边缘内缩 0.18 个 SVG 像素，Builder 为 0.12；终端提示符、
语言符号等细节只内缩 0.06 至 0.10。原本已较细的成对布局字形保留原样。
典型 2px 线条调整后约为 1.64px 或 1.76px，接近工作台的文字与细边框。

`weight_inset` 参数记录在 `sources.json` 中，单个字形可覆盖所属字体的默认值。
它使用 16 × 16 SVG 画布中的像素单位，不是屏幕像素；实际显示随 VS Code 尺寸
缩放。字体仍是 Regular 静态字形，改变 CSS 的 `font-weight` 不会完成这种调整。
内缩扩大孔洞而保留中心线；生成器会拒绝字形消失或面积损失过大的参数。

`rendered/<字体标识>/<码点>.svg` 由生成后的真实字体轮廓导出，陈列缩略图使用
这些文件；原始 SVG 仍保留在 `imported/`，哈希不变。衍生字体与预览保留各来源
的许可和署名。调整参数后重新生成字体、导出轮廓并刷新陈列缩略图。

未覆盖的产品图标继续使用 VS Code 默认 Codicons；不同的断点验证状态没有强行
合并为同一符号。汉堡菜单保持原生位置。本项目原有的 `symbolic/`、
`adwcode-symbols.ttf` 为 AGPL-3.0-or-later 自有字形，保留作为备用源资产，
当前主题不再引用该字体。

## 再生成

从仓库或随附资产目录再生成导入字体，需要 fontTools 和 skia-pathops，
后者用于描边展开、画布裁切与字重调整。
不需要安装 GNOME
Builder、MoreWaita 或系统图标主题，也不需要在构建时访问网络：

```sh
uv venv --python 3.14 /tmp/adwcode-icons
uv pip install --python /tmp/adwcode-icons/bin/python fonttools skia-pathops
/tmp/adwcode-icons/bin/python product-icons/build_imported.py
```

`build_imported.py` 根据来源记录校验 SVG 哈希，为不同许可证分别生成字体，
将署名与许可写入字体名称表，并导出 `rendered/` 中的陈列轮廓。
字体使用固定时间戳；备用字体也可再生成。
同一份输入应得到相同的字体。
更新资产时先核对上游图标许可，再更新原文件、提交号、哈希与映射，最后再生成
字体并检查 Git 差异。

原布局字体的再生成方式保持原样（需要 nanoemoji，其中未使用的窗口字形仍保留）：

```sh
nanoemoji --color_format glyf_colr_1 --family adwaita-icons \
  --output_file product-icons/adwaita-icons.ttf product-icons/scalable/*.svg
```

备用自有字体通过 `build_symbols.py` 再生成，同样仅依赖 fontTools。
可选资产工具的 C 扩展可能重新启用 GIL；它们不属于 Meson 的自由线程开发检查。
最终扩展直接读取随附的 TTF，不加载 Python 或字体生成依赖。
许可证登记见 [第三方许可证](../docs/01-第三方许可证.md)。
