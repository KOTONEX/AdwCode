# 扩展图标

扩展图标直接使用 **libadwaita 原版徽标**：蓝绿渐变圆形与三个白色三角折面。
SVG 保持上游原始字节，PNG 仅作等比导出。

## 文件与展示

- `adwcode.svg`：上游原始 SVG，标称尺寸为 48 × 48；与
  `upstream/libadwaita.svg` 的字节一致。
- `adwcode.png`：512 × 512 透明 PNG，仅用于扩展图标。
- `package.json` 的 `icon` 指向 PNG。打包器同时写入 VSIX 的图标元数据与资源项，
  用于扩展列表及详情页；项目文档不展示徽标图片。

用 librsvg 从原始 SVG 再生成 PNG；日常构建和安装不需要 librsvg：

```sh
rsvg-convert --width 512 --height 512 --output assets/adwcode.png assets/adwcode.svg
meson test -C builddir --print-errorlogs
meson compile -C builddir package
```

更新上游版本时，同步两个 SVG 文件、来源版本与哈希，再生成 PNG，一起提交。
保留原版的颜色、形状与比例，在浅色和深色背景下检查小尺寸展示。

## 许可与参考

徽标来自 libadwaita 的 `doc/libadwaita.svg`，署名为 The GNOME Project，保留
LGPL-2.1-or-later 许可；原始 SVG 和上游 LGPL-2.1 许可全文随附于 `upstream/`。
版本与文件哈希见 [来源记录](sources.json)，许可证登记见
[第三方许可证](../docs/01-第三方许可证.md)。

项目代码的 AGPL 许可不替代徽标的上游许可；采用徽标不表示官方背书。

- [libadwaita 上游徽标](https://gitlab.gnome.org/GNOME/libadwaita/-/blob/main/doc/libadwaita.svg)
- [VS Code 扩展清单中的图标字段](https://code.visualstudio.com/api/references/extension-manifest)
