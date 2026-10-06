// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 校验 package.json 的声明式默认设置、命令可用性与工作区能力，不加载扩展。
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const manifest = JSON.parse(fs.readFileSync(path.join(__dirname, '../package.json'), 'utf8'));
const contributes = manifest.contributes;
const defaults = contributes.configurationDefaults;
const commands = contributes.commands.map((entry) => entry.command);
const palette = contributes.menus.commandPalette;

// 默认值必须引用已注册的主题与产品图标；界面字体仍是可配置项。
const themeLabels = new Set(contributes.themes.map((theme) => theme.label));
for (const key of ['workbench.preferredLightColorTheme','workbench.preferredDarkColorTheme','workbench.preferredHighContrastLightColorTheme','workbench.preferredHighContrastColorTheme']) {
  assert.ok(themeLabels.has(defaults[key]), key);
}
assert.ok(contributes.productIconThemes.some((theme) => theme.id === defaults['workbench.productIconTheme']));
assert.equal(defaults['window.menuBarVisibility'],'compact');
assert.equal(defaults['editor.fontFamily'],'Adwaita Mono, monospace');
assert.ok(!('workbench.iconTheme' in defaults), '不能用默认值清空用户图标主题');
assert.equal(contributes.configuration.properties['adwcode.界面字体'].default,'');

// 写入用户设置的命令已移除，其余命令与命令面板入口只在 Linux 提供。
const expected = ['adwcode.查看外观安装状态','adwcode.安装GNOME外观','adwcode.安装仅关闭窗口控件'];
assert.ok(!commands.includes('adwcode.应用推荐设置'));
assert.ok(!commands.includes('adwcode.恢复推荐设置'));
assert.deepEqual([...commands].sort(), [...expected].sort());
for (const entry of contributes.commands) {
  assert.equal(entry.enablement,'isLinux',entry.command);
}
assert.deepEqual(palette.map((entry) => entry.command).sort(), [...expected].sort());
for (const entry of palette) {
  assert.equal(entry.when,'isLinux',entry.command);
}

// 扩展只在本机 UI 侧运行，不进入虚拟工作区与受限模式。
assert.equal(manifest.capabilities.virtualWorkspaces,false);
assert.equal(manifest.capabilities.untrustedWorkspaces.supported,false);
assert.deepEqual(manifest.extensionKind,['ui']);
console.log('清单：默认设置、命令与能力声明测试通过');
