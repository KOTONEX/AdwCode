# 产品图标主题

`adwaita.json` 与 `adwaita-icons.ttf` 让 `window.controlsStyle` 为 `custom` 时的
窗口控制按钮呈现 Adwaita 风格。

- **来源**：SVG 字形（`scalable/*.svg`）与字体由
  [piousdeer/vscode-adwaita](https://github.com/piousdeer/vscode-adwaita)
  生成，该仓库以 GPL-3.0-only 发布；字形本身派生自 GNOME 的符号图标。
- **许可证**：GPL-3.0-only。依 GPL-3.0 §13 与 AGPL-3.0 兼容，本项目整体按
  AGPL-3.0-or-later 发布（见仓库根目录的 `LICENSE`）。
- **重新生成**（需要 [nanoemoji](https://github.com/googlefonts/nanoemoji)）：

  ```sh
  nanoemoji --color_format glyf_colr_1 --family adwaita-icons \
    --output_file product-icons/adwaita-icons.ttf product-icons/scalable/*.svg
  ```

只覆盖四个字形（`chrome-close`、`chrome-maximize`、`chrome-minimize`、
`chrome-restore`）；标题栏布局按钮、菜单栏溢出与导航箭头刻意保留系统 codicon，
因为 Adwaita 实心符号字形在这些位置明显偏粗。
