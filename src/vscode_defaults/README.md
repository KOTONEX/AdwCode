# VS Code 默认语法高亮数据

`dark.json` 与 `light.json` 是 VS Code 内置「2026 Dark」/「2026 Light」主题解析后的
`tokenColors`，来源为 [microsoft/vscode](https://github.com/microsoft/vscode) 的
`extensions/theme-defaults/themes/`（MIT 许可证），供「默认语法高亮」主题变体使用。

用 `python3.14t src/update_defaults.py` 重新生成。
