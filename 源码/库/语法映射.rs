// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 自有贡献另可选择 LicenseRef-MulanPubL-2.0-or-later；第三方映射仍受 GPL-3.0-only 约束。
//! 基于 GNOME Builder 的 GtkSourceView 样式方案的语法高亮。
//!
//! 下面的映射派生自 piousdeer/vscode-adwaita
//! （<https://github.com/piousdeer/vscode-adwaita>，GPL-3.0-only），并适配到
//! `GtkSourceView方案/` 中存放的当前 GtkSourceView 5 方案：
//!
//! - `def:keyword` 更名为 `def:statement`；两个名字都接受。
//! - 新增 `def:emphasis`、`def:identifier`、`def:inline-code`、
//!   `def:note` 与 `def:deletion`。

use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::Event;

use crate::有序映射::有序映射;
use crate::调色板::{编辑器方案, 转为十六进制};
use crate::错误::{工具错误, 结果};

/// GtkSourceView `style` 元素解析后的颜色与字体样式。
#[derive(Clone, Debug)]
pub struct 样式信息 {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub font_style: String,
}

/// 解析后的 GtkSourceView 方案。
#[derive(Default)]
pub struct 样式方案 {
    pub named: BTreeMap<String, Option<String>>,
    pub styles: BTreeMap<String, 样式信息>,
}

/// TextMate 规则：Builder 生成，或随附的 VS Code 默认主题。
#[derive(Clone, Debug)]
pub struct 语法规则 {
    pub scope: Vec<String>,
    pub settings: 有序映射,
}

/// GtkSourceView 样式名 -> TextMate 作用域，顺序即输出顺序。
pub static 映射表: &[(&str, &[&str])] = &[
    // 默认颜色
    (
        "text",
        &[
            // 空选择器作用于所有内容
            "",
            // 内嵌表达式（例如字符串中的 ${→something←}）
            "meta.embedded",
            // 显式使用默认色的变量
            "variable",
            // XML 属性中的内嵌表达式标点
            "meta.tag.attributes punctuation.section.embedded",
            // 大多数运算符为符号型
            "keyword.operator",
            "storage.type.function.arrow", // =>
            // YAML 符号关键字
            "keyword.control.flow.block-scalar.literal",
            "keyword.control.flow.block-scalar.folded",
            "storage.modifier.chomping-indicator",
            // 字符串前缀（例如 Python 的 f、b、r）
            "storage.type.string",
            "string.quoted.byte.raw",
            // Rust
            "meta.macro.rules entity.name.function.macro.rust",
        ],
    ),
    (
        "def:base-n-integer",
        &[
            "constant.numeric.binary",
            "constant.numeric.octal",
            "constant.numeric.hex",
            "keyword.other.unit.binary",
            "keyword.other.unit.octal",
            "keyword.other.unit.hexadecimal",
            "keyword.other.unit.imaginary",
            "keyword.other.unit.exponent",
        ],
    ),
    (
        "def:boolean",
        &["constant.language.boolean", "constant.language.bool"],
    ),
    (
        "def:comment",
        &[
            "comment",
            // YAML
            "entity.other.document.begin.yaml",
            "entity.other.document.end.yaml",
        ],
    ),
    (
        "def:constant",
        &[
            "constant.language",
            // 字符（例如 Rust 中）
            "string.quoted.single.char",
            // { →"key"←: ... }（例如 JSON 中）
            "support.type.property-name",
            // CSS
            "support.constant.property-value.css",
            "source.css keyword.other.unit",
        ],
    ),
    (
        "def:decimal",
        &[
            "constant.numeric",
            "constant.numeric entity.name.type.numeric", // 1→i64← （例如 Rust）
        ],
    ),
    (
        "def:deletion",
        &["markup.strikethrough", "markup.strikethrough.markdown"],
    ),
    ("def:doc-comment-element", &["comment.block.documentation"]),
    ("def:emphasis", &["markup.italic", "markup.italic.markdown"]),
    ("def:floating-point", &["constant.numeric.float"]),
    // 注意：GtkSourceView 对 def:function 的应用并不一致（仅 Python 定义处生效），
    // 因此函数保持默认色，与上游一致。
    ("def:function", &[]),
    ("def:heading", &["markup.heading.markdown"]),
    (
        "def:identifier",
        &[
            "variable.other",
            "variable.other.readwrite",
            "variable.other.object",
            "meta.definition.variable",
            "support.variable",
        ],
    ),
    (
        "def:inline-code",
        &[
            "markup.inline.raw",
            "markup.raw.inline",
            "markup.inline.raw.string.markdown",
        ],
    ),
    (
        "def:statement",
        &[
            // 大多数关键字（运算符在 `text` 中不着色）
            "keyword",
            // 字母型运算符与关键字
            "keyword.operator.new",
            "keyword.operator.logical.python",
            "source.js keyword.operator.expression",
            "source.ts keyword.operator.expression",
            "storage.modifier",
            "storage.type.class",
            "storage.type.function",
            // YAML 键名属于标签名
            "entity.name.tag.yaml",
            // 针对用 `storage.type` 标记关键字的扩展的兼容处理
            "source.js storage.type",
            "source.ts storage.type",
            "source.tsx storage.type",
            "source.rust storage.type",
        ],
    ),
    ("def:number", &["constant.numeric"]),
    (
        "def:note",
        &["keyword.codetag.notation", "comment.line.note"],
    ),
    (
        "def:preprocessor",
        &[
            "meta.preprocessor",
            "meta.preprocessor keyword.control",
            "punctuation.decorator",
            "meta.decorator entity.name.function",
            "entity.name.function.decorator",
            "keyword.control.at-rule.media",
            "constant.character.entity",
            "punctuation.section.embedded",
            "punctuation.definition.template-expression",
        ],
    ),
    ("def:shebang", &["comment.line.number-sign.shebang"]),
    ("def:special-char", &["constant.character.escape"]),
    ("def:string", &["string"]),
    ("def:strong-emphasis", &["markup.bold.markdown"]),
    (
        "def:type",
        &[
            "storage.type",
            "entity.name.type",
            "entity.name.namespace",
            "keyword.type.cs",
            "support.type",
            "support.class.builtin",
            "support.class.promise",
        ],
    ),
    ("def:underlined", &[]),
    ("def:warning", &[]),
    // C#
    ("c-sharp:format", &[]),
    ("c-sharp:preprocessor", &["meta.preprocessor.cs"]),
    // C
    ("c:printf", &["constant.other.placeholder"]),
    ("c:signal-name", &[]),
    ("c:storage-class", &["source.c storage.modifier"]),
    ("c:type-keyword", &["source.c storage.type"]),
    // CSS
    ("css:id-selector", &["entity.other.attribute-name.id.css"]),
    ("css:property-name", &["support.type.property-name.css"]),
    (
        "css:pseudo-selector",
        &[
            "entity.other.attribute-name.pseudo-element.css",
            "entity.other.attribute-name.pseudo-class.css",
            "meta.selector.css punctuation.section.function",
        ],
    ),
    (
        "css:selector-symbol",
        &[
            "meta.selector.css keyword.operator",
            "entity.other.attribute-name.css",
        ],
    ),
    ("css:type-selector", &[]),
    (
        "css:vendor-specific",
        &["support.type.vendored.property-name.css"],
    ),
    // Diff
    ("diff:added-line", &["markup.inserted.diff"]),
    ("diff:changed-line", &["markup.changed"]),
    ("diff:diff-file", &["meta.diff.header"]),
    ("diff:location", &["meta.diff.range"]),
    ("diff:removed-line", &["markup.deleted.diff"]),
    // Go
    ("go:printf", &["constant.other.placeholder.go"]),
    // Python
    (
        "python:builtin-function",
        &["support.function.builtin.python"],
    ),
    ("python:class-name", &["entity.name.type.class.python"]),
    ("python:module-handler", &["keyword.control.import.python"]),
    // Rust
    (
        "rust:attribute",
        &[
            "meta.attribute.rust",
            "meta.attribute.rust keyword.operator",
        ],
    ),
    ("rust:lifetime", &["entity.name.type.lifetime.rust"]),
    ("rust:macro", &["entity.name.function.macro"]),
    // XML
    (
        "xml:attribute-name",
        &[
            "meta.tag entity.other.attribute-name",
            "meta.tag keyword.operator.assignment",
            "punctuation.separator.key-value.html",
            "punctuation.separator.key-value.svelte",
            "text.xml meta.tag",
        ],
    ),
    ("xml:attribute-value", &["meta.tag string"]),
    (
        "xml:element-name",
        &[
            "entity.name.tag",
            "support.class.component.svelte",
            "punctuation.definition.tag",
        ],
    ),
    ("xml:namespace", &[]),
    (
        "xml:processing-instruction",
        &[
            "text.xml meta.tag.preprocessor entity.name.tag",
            "text.xml meta.tag.preprocessor punctuation.definition.tag",
        ],
    ),
];

/// 预期在当前方案中不存在的样式。
pub static 容忍缺失: &[&str] = &["def:keyword", "c-sharp:format", "diff:changed-line"];

/// 解析随附的 GtkSourceView 方案并解析其中的具名颜色。
pub fn 加载样式方案(根目录: &std::path::Path, mode: &str) -> 结果<样式方案> {
    if !matches!(mode, "dark" | "light") {
        return Err(工具错误::新(format!("未知主题模式：{mode}")));
    }
    let 文件 = 根目录
        .join("源码")
        .join("GtkSourceView方案")
        .join(if mode == "dark" {
            "Adwaita-dark.xml"
        } else {
            "Adwaita.xml"
        });
    let 文本 = std::fs::read_to_string(&文件).map_err(|错误| {
        工具错误::带来源(format!("无法读取样式方案：{}", 文件.display()), 错误)
    })?;
    let mut reader = Reader::from_str(&文本);
    let mut 具名颜色: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut 原始样式: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(元素)) | Ok(Event::Empty(元素)) => {
                let 名称 = String::from_utf8_lossy(元素.name().as_ref()).to_string();
                if 名称 != "color" && 名称 != "style" {
                    continue;
                }
                let mut 属性: BTreeMap<String, String> = BTreeMap::new();
                for 属性项 in 元素.attributes() {
                    let 属性项 = 属性项
                        .map_err(|错误| 工具错误::新(format!("XML 属性解析失败：{错误}")))?;
                    let 键 = String::from_utf8_lossy(属性项.key.as_ref()).to_string();
                    let 值 = 属性项
                        .decoded_and_normalized_value(XmlVersion::default(), reader.decoder())
                        .map_err(|错误| 工具错误::新(format!("XML 属性解码失败：{错误}")))?
                        .to_string();
                    属性.insert(键, 值);
                }
                if 名称 == "color" {
                    if let Some(名字) = 属性.get("name") {
                        具名颜色.insert(名字.clone(), 属性.get("value").cloned());
                    }
                } else if let Some(名字) = 属性.get("name") {
                    原始样式.insert(名字.clone(), 属性);
                }
            }
            Ok(Event::Eof) => break,
            Err(错误) => {
                return Err(工具错误::新(
                    format!("{} 解析失败：{错误}", 文件.display()),
                ));
            }
            _ => {}
        }
    }

    fn 解析值(
        值: &str,
        具名颜色: &BTreeMap<String, Option<String>>,
        文件名: &str,
    ) -> 结果<Option<String>> {
        let mut 当前 = 值;
        let mut 已访问 = std::collections::BTreeSet::new();
        loop {
            if !已访问.insert(当前) {
                return Err(工具错误::新(format!(
                    "{文件名} 中存在循环颜色引用 {当前:?}"
                )));
            }
            if let Some((r, g, b, a)) = 解析rgba(当前) {
                return Ok(Some(转为十六进制(r, g, b, a)));
            }
            if let Some(十六进制) = 当前.strip_prefix('#') {
                if matches!(十六进制.len(), 6 | 8)
                    && 十六进制.bytes().all(|字符| 字符.is_ascii_hexdigit())
                {
                    return Ok(Some(当前.to_lowercase()));
                }
                return Err(工具错误::新(
                    format!("{文件名} 中存在无效颜色 {当前:?}"),
                ));
            }
            match 具名颜色.get(当前) {
                Some(Some(目标)) => 当前 = 目标,
                Some(None) => return Ok(None),
                None => {
                    return Err(工具错误::新(format!(
                        "{文件名} 中存在未知颜色名称 {当前:?}"
                    )));
                }
            }
        }
    }

    let 文件名 = 文件.file_name().map_or_else(
        || 文件.display().to_string(),
        |名字| 名字.to_string_lossy().to_string(),
    );
    let mut styles: BTreeMap<String, 样式信息> = BTreeMap::new();
    for (名字, 属性) in &原始样式 {
        let 字体样式: Vec<&str> = ["italic", "bold", "underline", "strikethrough"]
            .into_iter()
            .filter(|键| matches!(属性.get(*键).map(String::as_str), Some("true" | "1")))
            .collect();
        let 前景 = match 属性.get("foreground") {
            Some(值) => 解析值(值, &具名颜色, &文件名)?,
            None => None,
        };
        let 背景 = match 属性.get("background") {
            Some(值) => 解析值(值, &具名颜色, &文件名)?,
            None => None,
        };
        styles.insert(
            名字.clone(),
            样式信息 {
                foreground: 前景,
                background: 背景,
                font_style: 字体样式.join(" "),
            },
        );
    }
    Ok(样式方案 {
        named: 具名颜色,
        styles,
    })
}

/// 匹配 `#rgba(1, 2, 3, 0.5)`；不匹配返回 `None`。
fn 解析rgba(值: &str) -> Option<(f64, f64, f64, f64)> {
    let 剩余 = 值.strip_prefix("#rgba(")?;
    let 内部 = 剩余.strip_suffix(')')?;
    let 部分: Vec<&str> = 内部.split(',').map(str::trim).collect();
    if 部分.len() != 4 || 部分.iter().any(|项| 项.is_empty()) {
        return None;
    }
    let 数值: Vec<f64> = 部分
        .iter()
        .map(|项| 项.parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;
    if 数值.iter().any(|数| !数.is_finite())
        || 数值[..3].iter().any(|数| !(0.0..=255.0).contains(数))
        || !(0.0..=1.0).contains(&数值[3])
    {
        return None;
    }
    Some((数值[0].trunc(), 数值[1].trunc(), 数值[2].trunc(), 数值[3]))
}

/// 供调色板使用的编辑器表面颜色。
pub fn 编辑器颜色(根目录: &std::path::Path, mode: &str) -> 结果<编辑器方案> {
    let styles = 加载样式方案(根目录, mode)?.styles;
    let 取 = |名称: &str, 字段: fn(&样式信息) -> Option<String>| -> Option<String> {
        styles.get(名称).and_then(字段)
    };
    Ok(编辑器方案 {
        text_bg: 取("text", |样式| 样式.background.clone()),
        text_fg: 取("text", |样式| 样式.foreground.clone()),
        current_line: 取("current-line", |样式| 样式.background.clone()),
        cursor: 取("cursor", |样式| 样式.foreground.clone()),
        line_numbers_bg: 取("line-numbers", |样式| 样式.background.clone()),
        line_numbers_fg: 取("line-numbers", |样式| 样式.foreground.clone()),
        search_match_bg: 取("search-match", |样式| 样式.background.clone()),
        search_match_fg: 取("search-match", |样式| 样式.foreground.clone()),
        background_pattern: 取("background-pattern", |样式| 样式.background.clone()),
    })
}

/// Builder 语法高亮的 TextMate 规则。
pub fn 语法颜色(根目录: &std::path::Path, mode: &str) -> 结果<Vec<语法规则>> {
    let styles = 加载样式方案(根目录, mode)?.styles;
    let mut rules = Vec::new();
    for (样式名, scopes) in 映射表 {
        if scopes.is_empty() {
            continue;
        }
        let Some(样式) = styles.get(*样式名) else {
            if !容忍缺失.contains(样式名) {
                eprintln!("警告：样式 {样式名:?} 不在方案中（{mode}）");
            }
            continue;
        };
        let mut settings = 有序映射::新();
        settings.放("fontStyle", 样式.font_style.clone());
        if let Some(前景) = &样式.foreground {
            settings.放("foreground", 前景.clone());
        }
        if let Some(背景) = &样式.background {
            settings.放("background", 背景.clone());
        }
        rules.push(语法规则 {
            scope: scopes.iter().map(|scope| (*scope).to_string()).collect(),
            settings,
        });
    }
    Ok(rules)
}

/// 映射到 Builder 方案颜色的语义高亮；顺序即输出顺序。
pub fn 语义标记颜色(
    根目录: &std::path::Path,
    mode: &str,
) -> 结果<Vec<(String, Option<String>)>> {
    let styles = 加载样式方案(根目录, mode)?.styles;
    let 取色 = |名称: &str, 回退: Option<&str>| -> Option<String> {
        styles
            .get(名称)
            .and_then(|样式| 样式.foreground.clone())
            .or_else(|| 回退.map(str::to_string))
    };
    let 类型色 = 取色("def:type", None);
    let 标识符 = 取色("def:identifier", None);
    let 项 = |名称: &str, 值: Option<String>| (名称.to_string(), 值);
    Ok(vec![
        项("namespace", 标识符.clone()),
        项("type", 类型色.clone()),
        项("class", 类型色.clone()),
        项("enum", 类型色.clone()),
        项("interface", 类型色.clone()),
        项("struct", 类型色.clone()),
        项("typeParameter", 类型色.clone()),
        项("function", 取色("def:function", None)),
        项("method", 取色("def:function", None)),
        项("property", 标识符.clone()),
        项("variable", 标识符.clone()),
        项("parameter", 标识符.clone()),
        项("macro", 取色("def:preprocessor", None)),
        项("keyword", 取色("def:statement", None)),
        项("comment", 取色("def:comment", None)),
        项("string", 取色("def:string", None)),
        项("number", 取色("def:number", None)),
        项("regexp", 取色("def:special-char", None)),
        项("operator", 取色("text", None)),
        项("decorator", 取色("def:preprocessor", None)),
        项("deprecated", 标识符),
    ])
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::path::Path;

    fn 根目录() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn 拒绝循环引用无效颜色和未知模式() {
        let 临时 = tempfile::tempdir().unwrap();
        let 目录 = 临时.path().join("源码/GtkSourceView方案");
        std::fs::create_dir_all(&目录).unwrap();
        for 颜色 in [
            "<color name=\"a\" value=\"b\"/><color name=\"b\" value=\"a\"/>",
            "<color name=\"a\" value=\"#rgba(NaN,0,0,1)\"/>",
            "<color name=\"a\" value=\"#rgba(256,0,0,1)\"/>",
            "<color name=\"a\" value=\"#nope\"/>",
        ] {
            std::fs::write(
                目录.join("Adwaita.xml"),
                format!(
                    "<style-scheme>{颜色}<style name=\"text\" foreground=\"a\"/></style-scheme>"
                ),
            )
            .unwrap();
            assert!(加载样式方案(临时.path(), "light").is_err());
        }
        assert!(加载样式方案(根目录(), "不存在").is_err());
    }

    #[test]
    fn 与金样例一致() {
        let 路径 = concat!(env!("CARGO_MANIFEST_DIR"), "/测试/Rust/语法样例.json");
        let 文本 = std::fs::read_to_string(路径).expect("读取语法金样例");
        let 样例: serde_json::Value = serde_json::from_str(&文本).expect("解析语法金样例");
        for mode in ["dark", "light"] {
            let 期望 = &样例[mode];
            let 规则 = 语法颜色(根目录(), mode).unwrap();
            let 规则_json: Vec<serde_json::Value> = 规则
                .iter()
                .map(|规则| {
                    let mut 设置 = serde_json::Map::new();
                    for (键, 值) in 规则.settings.条目() {
                        设置.insert(键.clone(), serde_json::Value::String(值.clone()));
                    }
                    let mut 对象 = serde_json::Map::new();
                    对象.insert(
                        "scope".to_string(),
                        serde_json::Value::Array(
                            规则
                                .scope
                                .iter()
                                .map(|值| serde_json::Value::String(值.clone()))
                                .collect(),
                        ),
                    );
                    对象.insert("settings".to_string(), serde_json::Value::Object(设置));
                    serde_json::Value::Object(对象)
                })
                .collect();
            assert_eq!(
                serde_json::Value::Array(规则_json),
                期望["tokenColors"],
                "{mode} tokenColors"
            );

            let 语义 = 语义标记颜色(根目录(), mode).unwrap();
            let mut 语义_json = serde_json::Map::new();
            for (名称, 值) in 语义 {
                语义_json.insert(
                    名称,
                    match 值 {
                        Some(值) => serde_json::Value::String(值),
                        None => serde_json::Value::Null,
                    },
                );
            }
            assert_eq!(
                serde_json::Value::Object(语义_json),
                期望["semanticTokenColors"],
                "{mode} semanticTokenColors"
            );

            let 方案 = 编辑器颜色(根目录(), mode).unwrap();
            let mut 方案_json = serde_json::Map::new();
            for (键, 值) in [
                ("text_bg", 方案.text_bg),
                ("text_fg", 方案.text_fg),
                ("current_line", 方案.current_line),
                ("cursor", 方案.cursor),
                ("line_numbers_bg", 方案.line_numbers_bg),
                ("line_numbers_fg", 方案.line_numbers_fg),
                ("search_match_bg", 方案.search_match_bg),
                ("search_match_fg", 方案.search_match_fg),
                ("background_pattern", 方案.background_pattern),
            ] {
                方案_json.insert(
                    键.to_string(),
                    match 值 {
                        Some(值) => serde_json::Value::String(值),
                        None => serde_json::Value::Null,
                    },
                );
            }
            assert_eq!(
                serde_json::Value::Object(方案_json),
                期望["编辑器颜色"],
                "{mode} 编辑器颜色"
            );
        }
    }
}
