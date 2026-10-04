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

## 覆盖范围

- 窗口关闭、最大化、最小化和还原保留上游 Adwaita 字形。
- 布局按钮使用随附的左右侧栏和底部面板字形，开启与关闭状态保持配对。
- 文件、搜索、版本控制、运行与调试、扩展、终端、设置、刷新和新增使用
  本项目的单色符号，位于 `symbolic/`，以 AGPL-3.0-or-later 发布。
- 未覆盖的产品图标继续使用 VS Code 默认字形，文件类型图标不受产品图标主题影响。

自有字形由 `build_symbols.py` 用 fontTools 生成，只有重新生成字体时需要该依赖：

```sh
uv venv /tmp/adwcode-icons
uv pip install --python /tmp/adwcode-icons/bin/python fonttools
/tmp/adwcode-icons/bin/python product-icons/build_symbols.py
```

该可选资产工具的 C 扩展可能重新启用 GIL；不属于 Meson 的自由线程开发检查。
最终扩展直接读取已提交的 TTF，不加载 Python 或字体生成依赖。
同一源文件生成的字体使用固定时间戳，可用 Git 差异核对再生成结果。
