#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""把扩展打包为 .vsix（带 VS Code 清单的 zip）。

无需 Node.js：归档结构与 `vsce package` 生成的一致。
"""

from __future__ import annotations

import json
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

ROOT: Path = Path(__file__).parent.parent

CONTENT_TYPES: str = """<?xml version="1.0" encoding="utf-8"?>
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
"""

MANIFEST: str = """<?xml version="1.0" encoding="utf-8"?>
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
    <License>extension/LICENSE</License>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.License" Path="extension/LICENSE" Addressable="true" />
  </Assets>
</PackageManifest>
"""

INCLUDE: list[str] = ["package.json", "README.md", "LICENSE", "extension", "themes", "product-icons", "extras", "docs", "CONTRIBUTING.md", "CHANGELOG.md", "AGENTS.md", "src/vscode_defaults/README.md"]
SKIP_SUFFIXES: set[str] = {".pyc", ".py"}
ASSET_BUILD_SCRIPTS = {"build_symbols.py", "build_imported.py"}


def collect() -> list[Path]:
    files: list[Path] = []
    for item in INCLUDE:
        path = ROOT / item
        if path.is_file():
            files.append(path)
        elif path.is_dir():
            files.extend(p for p in sorted(path.rglob("*")) if p.is_file())
    # 字体连同对应 SVG、来源记录与再生成脚本一起分发；它们不参与扩展运行。
    return [path for path in files if "__pycache__" not in path.parts and (
        path.suffix not in SKIP_SUFFIXES
        or (path.parent == ROOT / "product-icons" and path.name in ASSET_BUILD_SCRIPTS)
    )]


def main() -> None:
    manifest = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    name = manifest["name"]
    version = manifest["version"]
    output = ROOT / f"{name}-{version}.vsix"
    def xml(value: str) -> str:
        return escape(value, {'"': "&quot;", "'": "&apos;"})

    vsix_manifest = MANIFEST.format(
        name=xml(name),
        version=xml(version),
        publisher=xml(manifest["publisher"]),
        display_name=xml(manifest["displayName"]),
        description=xml(manifest["description"]),
        keywords=xml(",".join(manifest.get("keywords", []))),
        categories=xml(",".join(manifest.get("categories", []))),
        engine=xml(manifest["engines"]["vscode"]),
    )
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", CONTENT_TYPES)
        archive.writestr("extension.vsixmanifest", vsix_manifest)
        for path in collect():
            archive.write(path, f"extension/{path.relative_to(ROOT)}")
    print(f"已生成 {output.relative_to(ROOT)} ({output.stat().st_size / 1024:.0f} KiB)")


if __name__ == "__main__":
    main()
