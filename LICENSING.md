# 许可声明与分发边界

版权声明：2026 AdwCode contributors。以下授权仅适用于本项目有权许可的自有内容。
`OR` 表示使用者可以选择其中一种许可，并遵守所选许可的全部条款。

## 自有代码

自有代码采用 **AGPL-3.0-or-later 或木兰公共许可证第 2 版或其后续版本**：

```text
AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
```

范围包含 Python、JavaScript、类型声明、CSS、构建脚本、测试、CI 工作流与开发
配置中的自有内容。支持注释的自有代码文件在文件头标注该表达式；JSON 配置等
无法添加注释的内容依据本声明授权，混合内容按下文保留第三方约束。

- AGPL-3.0 全文：[LICENSE](LICENSE)，也可选择其后续版本。
- 木兰公共许可证第 2 版全文：[LicenseRef-MulanPubL-2.0.txt](LICENSES/LicenseRef-MulanPubL-2.0.txt)。

`LicenseRef-MulanPubL-2.0-or-later` 是本项目的自定义 SPDX 许可标识，明确对应木兰**公共**
许可证第 2 版，不是木兰宽松许可证 `MulanPSL-2.0`。全文取自
[许可作者提交的中英文文本](https://lists.opensource.org/pipermail/license-review_lists.opensource.org/2025-January/005620.html)，
原许可地址为 <http://license.coscl.org.cn/MulanPubL-2.0>。

## 自有资产

自有资产采用 **AGPL-3.0-or-later 或 CC BY-SA 4.0 或其后续版本**：

```text
AGPL-3.0-or-later OR CC-BY-SA-4.0+
```

范围包含自有的项目文档、说明与模板、扩展徽标、`product-icons/symbolic/` 字形
及其 `adwcode-symbols.ttf` 字体、自有产品图标映射、性能基准数据，以及生成主题
中的自有贡献。署名为 AdwCode contributors；修改和分发时遵守所选许可。
CC BY-SA 4.0 全文见 [assets/CC-BY-SA-4.0.txt](assets/CC-BY-SA-4.0.txt)，
AGPL 全文见 [LICENSE](LICENSE)。扩展徽标声明另见 [assets/LICENSE](assets/LICENSE)。

## 第三方与混合内容

第三方代码、数据、字体、字形及其衍生资产保留原许可，不能仅因复制、渲染、
转换为字体或放入本仓库而获得上述额外授权。许可证原文也不属于自有资产。
完整来源与分发边界见 [第三方许可证](docs/01-第三方许可证.md)。

- `src/tokens.py` 的 TextMate 映射派生自 GPL-3.0-only 上游；其自有贡献提供
  上述代码双重许可，第三方部分仍受 GPL 约束。当前组合文件按 AGPL 分支分发，
  不能将整个文件仅改标为木兰许可。
- `src/gtksourceview_xml/`、`src/vscode_defaults/` 保留其 LGPL、MIT 许可。
  生成主题中的相应数据和 GPL 映射不会因本项目的自有贡献授权而重新许可。
- `product-icons/scalable/`、`adwaita-icons.ttf`、导入的 SVG、转换字体、
  `product-icons/rendered/` 及其文档缩略图均保留对应上游许可。

重新使用或组合混合内容时，须分别遵守相关第三方条款；自有贡献的可选许可不
替代整个组合作品的分发要求。本声明不授予第三方商标权利。

## 后续版本授权

本项目对有权许可的自有内容明确允许选择木兰公共许可证第 2 版或其维护方
正式发布的后续版本，以及 CC BY-SA 4.0 或 Creative Commons 正式发布的该许可
后续版本。`LicenseRef-MulanPubL-2.0-or-later` 表示前者，`CC-BY-SA-4.0+` 表示
后者。此额外授权不修改随附的许可证原文，也不扩展到第三方内容。
