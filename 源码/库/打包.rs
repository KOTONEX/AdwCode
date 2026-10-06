// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 把扩展打包为 .vsix（带 VS Code 清单的 zip）。
//!
//! 无需 Node.js：归档结构与 `vsce package` 生成的一致。
//! 默认从完整 Git 历史生成日志；无 Git 的源码副本可显式提供预先生成的日志。

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;
use zip::write::SimpleFileOptions;

use crate::错误::{工具错误, 结果};

const 内容类型: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json"/>
  <Default Extension="js" ContentType="application/javascript"/>
  <Default Extension="py" ContentType="text/x-python"/>
  <Default Extension="css" ContentType="text/css"/>
  <Default Extension="svg" ContentType="image/svg+xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="md" ContentType="text/markdown"/>
  <Default Extension="txt" ContentType="text/plain"/>
  <Default Extension="vsixmanifest" ContentType="text/xml"/>
  <Default Extension="xml" ContentType="text/xml"/>
  <Default Extension="ttf" ContentType="application/font-sfnt"/>
</Types>
"#;

const 清单模板: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Language="en-US" Id="{name}" Version="{version}" Publisher="{publisher}" />
    <DisplayName>{display_name}</DisplayName>
    <Description xml:space="preserve">{description}</Description>
    <Tags>{keywords}</Tags>
    <Categories>{categories}</Categories>
    <GalleryFlags>Public</GalleryFlags>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="{engine}" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionKind" Value="ui" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionDependencies" Value="" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionPack" Value="" />
      <Property Id="Microsoft.VisualStudio.Code.LocalizedLanguages" Value="" />
      <Property Id="Microsoft.VisualStudio.Code.EnabledApiProposals" Value="" />
      <Property Id="Microsoft.VisualStudio.Code.ExecutesCode" Value="true" />
    </Properties>
    <License>extension/许可声明.md</License>
{icon_metadata}
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.License" Path="extension/许可声明.md" Addressable="true" />
{icon_asset}
  </Assets>
</PackageManifest>
"#;

const 包含清单: [&str; 14] = [
    "package.json",
    "README.md",
    "LICENSE",
    "许可声明.md",
    "LICENSES",
    "资产",
    "扩展",
    "主题",
    "产品图标",
    "附加外观",
    "文档",
    "CONTRIBUTING.md",
    "AGENTS.md",
    "源码/VSCode默认数据/README.md",
];

const 跳过后缀: [&str; 2] = [".pyc", ".py"];
const 资产生成脚本: [&str; 2] = ["生成自有字形.py", "生成导入字形.py"];

/// 收集打包文件：字体连同对应 SVG、来源记录与再生成脚本一起分发。
pub fn 收集文件(根目录: &Path) -> 结果<Vec<PathBuf>> {
    let mut 文件: Vec<PathBuf> = Vec::new();
    for 项目 in 包含清单 {
        let 路径 = 根目录.join(项目);
        if 路径.is_file() {
            文件.push(路径);
        } else if 路径.is_dir() {
            let mut 目录内: Vec<PathBuf> = Vec::new();
            递归收集(&路径, &mut 目录内)?;
            目录内.sort_by(|左, 右| 左.to_string_lossy().cmp(&右.to_string_lossy()));
            文件.extend(目录内);
        }
    }
    let 产品目录 = 根目录.join("产品图标");
    Ok(文件
        .into_iter()
        .filter(|路径| {
            !路径
                .components()
                .any(|部件| 部件.as_os_str() == "__pycache__")
        })
        .filter(|路径| {
            let 后缀 = 路径
                .extension()
                .map_or_else(String::new, |扩展| format!(".{}", 扩展.to_string_lossy()));
            if 跳过后缀.contains(&后缀.as_str()) {
                let 名字 = 路径
                    .file_name()
                    .map_or_else(String::new, |名字| 名字.to_string_lossy().to_string());
                路径.parent() == Some(产品目录.as_path()) && 资产生成脚本.contains(&名字.as_str())
            } else {
                true
            }
        })
        .collect())
}

fn 递归收集(目录: &Path, 输出: &mut Vec<PathBuf>) -> 结果<()> {
    for 条目 in std::fs::read_dir(目录)? {
        let 条目 = 条目?;
        let 路径 = 条目.path();
        if 路径.is_dir() {
            递归收集(&路径, 输出)?;
        } else if 路径.is_file() {
            输出.push(路径);
        }
    }
    Ok(())
}

fn xml转义(值: &str) -> String {
    值.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// 构建 VSIX 到指定输出路径。
pub fn 构建(根目录: &Path, 输出: &Path, 变更日志路径: Option<&Path>) -> 结果<()> {
    let 清单文本 = std::fs::read_to_string(根目录.join("package.json"))?;
    let manifest: Value = serde_json::from_str(&清单文本)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    let name = manifest["name"].as_str().unwrap_or("");
    let version = manifest["version"].as_str().unwrap_or("");
    let 文件 = 收集文件(根目录)?;
    let mut 图标元数据 = String::new();
    let mut 图标资产 = String::new();
    if let Some(icon) = manifest.get("icon").and_then(Value::as_str) {
        let 图标路径 = PathBuf::from(icon);
        if 图标路径.is_absolute()
            || 图标路径.components().any(|部件| 部件.as_os_str() == "..")
            || !文件.contains(&根目录.join(&图标路径))
        {
            return Err(工具错误::新(
                "扩展图标必须是已纳入打包范围的仓库内文件",
            ));
        }
        let 图标地址 = xml转义(&format!("extension/{icon}"));
        图标元数据 = format!("    <Icon>{图标地址}</Icon>");
        图标资产 = format!(
            "    <Asset Type=\"Microsoft.VisualStudio.Services.Icons.Default\" Path=\"{图标地址}\" Addressable=\"true\" />"
        );
    }
    let 关键字 = manifest["keywords"]
        .as_array()
        .map(|列表| {
            列表
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    let 分类 = manifest["categories"]
        .as_array()
        .map(|列表| {
            列表
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    let 清单 = 清单模板
        .replace("{icon_metadata}", &图标元数据)
        .replace("{icon_asset}", &图标资产)
        .replace("{name}", &xml转义(name))
        .replace("{version}", &xml转义(version))
        .replace(
            "{publisher}",
            &xml转义(manifest["publisher"].as_str().unwrap_or("")),
        )
        .replace(
            "{display_name}",
            &xml转义(manifest["displayName"].as_str().unwrap_or("")),
        )
        .replace(
            "{description}",
            &xml转义(manifest["description"].as_str().unwrap_or("")),
        )
        .replace("{keywords}", &xml转义(&关键字))
        .replace("{categories}", &xml转义(&分类))
        .replace(
            "{engine}",
            &xml转义(manifest["engines"]["vscode"].as_str().unwrap_or("")),
        );
    // 正常打包读取完整 Git 历史；无 Git 的导出副本须显式提供已生成日志。
    let 日志 = if let Some(路径) = 变更日志路径 {
        std::fs::read_to_string(路径)?
    } else {
        crate::变更日志::生成变更日志(根目录)?
    };
    if 日志.trim().is_empty() {
        return Err(工具错误::新("用于打包的变更日志为空"));
    }
    if let Some(父目录) = 输出.parent() {
        std::fs::create_dir_all(父目录)?;
    }
    let 文件句柄 = std::fs::File::create(输出)?;
    let mut 归档 = zip::ZipWriter::new(文件句柄);
    let 选项 = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    归档
        .start_file("[Content_Types].xml", 选项)
        .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    归档
        .write_all(内容类型.as_bytes())
        .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    归档
        .start_file("extension.vsixmanifest", 选项)
        .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    归档
        .write_all(清单.as_bytes())
        .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    归档
        .start_file("extension/CHANGELOG.md", 选项)
        .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    归档
        .write_all(日志.as_bytes())
        .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    for 路径 in &文件 {
        let 相对 = 路径
            .strip_prefix(根目录)
            .map_err(|_| 工具错误::新("打包文件位于仓库之外"))?;
        let 条目名 = format!("extension/{}", 相对.to_string_lossy().replace('\\', "/"));
        归档
            .start_file(&条目名, 选项)
            .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
        let 内容 = std::fs::read(路径)?;
        归档
            .write_all(&内容)
            .map_err(|错误| 工具错误::新(format!("写入归档失败：{错误}")))?;
    }
    归档
        .finish()
        .map_err(|错误| 工具错误::新(format!("完成归档失败：{错误}")))?;
    Ok(())
}

/// `adwcode 打包` 的入口。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let mut 变更日志: Option<PathBuf> = None;
    let mut 序号 = 0;
    while 序号 < 参数.len() {
        match 参数[序号].as_str() {
            "--变更日志" => {
                序号 += 1;
                变更日志 = Some(PathBuf::from(
                    参数
                        .get(序号)
                        .ok_or_else(|| 工具错误::新("--变更日志 缺少取值"))?,
                ));
            }
            其他 => return Err(工具错误::新(format!("未知参数：{其他}"))),
        }
        序号 += 1;
    }
    let 清单文本 = std::fs::read_to_string(根目录.join("package.json"))?;
    let manifest: Value = serde_json::from_str(&清单文本)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    let 输出 = 根目录.join(format!(
        "{}-{}.vsix",
        manifest["name"].as_str().unwrap_or(""),
        manifest["version"].as_str().unwrap_or("")
    ));
    构建(根目录, &输出, 变更日志.as_deref())?;
    let 大小 = std::fs::metadata(&输出)?.len();
    let 相对 = 输出
        .strip_prefix(根目录)
        .map_or_else(|_| 输出.clone(), Path::to_path_buf);
    println!(
        "已生成 {} ({:.0} KiB)",
        相对.display(),
        (大小 as f64) / 1024.0
    );
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 打包包含条目且不含源码python() {
        let 根 = Path::new(env!("CARGO_MANIFEST_DIR"));
        let 输出 = std::env::temp_dir().join("adwcode-打包测试.vsix");
        let _ = std::fs::remove_file(&输出);
        构建(根, &输出, None).expect("构建 VSIX");
        let 文件句柄 = std::fs::File::open(&输出).expect("打开 VSIX");
        let mut 归档 = zip::ZipArchive::new(文件句柄).expect("读取 VSIX");
        let mut 条目: Vec<String> = (0..归档.len())
            .map(|编号| 归档.by_index(编号).expect("条目").name().to_string())
            .collect();
        条目.sort();
        for 必需 in [
            "[Content_Types].xml",
            "extension.vsixmanifest",
            "extension/CHANGELOG.md",
            "extension/package.json",
            "extension/扩展/扩展.js",
            "extension/产品图标/adwcode.json",
            "extension/主题/adwcode-深色.json",
            "extension/许可声明.md",
        ] {
            assert!(条目.contains(&必需.to_string()), "缺少 {必需}");
        }
        assert!(
            !条目
                .iter()
                .any(|名字| 名字.ends_with(".py") && !名字.contains("产品图标")),
            "不应包含源码 Python 文件"
        );
        assert!(条目.contains(&"extension/产品图标/生成导入字形.py".to_string()));
        let mut 清单 = String::new();
        use std::io::Read;
        归档
            .by_name("extension.vsixmanifest")
            .expect("清单条目")
            .read_to_string(&mut 清单)
            .expect("读取清单");
        assert!(清单.contains("Id=\"AdwCode\""));
        assert!(清单.contains(&format!("Version=\"{}\"", manifest版本(根))));
        drop(归档);
        let _ = std::fs::remove_file(&输出);
    }

    fn manifest版本(根: &Path) -> String {
        let 文本 = std::fs::read_to_string(根.join("package.json")).expect("读取清单");
        let manifest: Value = serde_json::from_str(&文本).expect("解析清单");
        manifest["version"].as_str().expect("版本").to_string()
    }
}
