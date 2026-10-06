// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
// 使用内存配置与命令模拟，验证重复应用和部分失败，不访问工作窗口。
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const values = new Map([['editor.fontFamily','原有代码字体'], ['adwcode.自动重载',true], ['workbench.productIconTheme','原有产品图标']]);
const messages = [];
let backup, failAt, errors=0, writes=0, unavailable, workspaceReload, monoFont;
const vscode = {
  ConfigurationTarget: {Global:1},
  workspace: {getConfiguration(section){return {
    get(name,fallback){if(section+'.'+name==='adwcode.自动重载'&&workspaceReload!==undefined)return workspaceReload;return values.has(section+'.'+name)?values.get(section+'.'+name):fallback},
    inspect(name){return {defaultValue:section+'.'+name===unavailable?undefined:null,globalValue:values.get(section+'.'+name)}},
    async update(name,value){
      assert.ok(backup, '配置写入之前必须保留备份');
      writes++;
      if (writes===1) assert.equal(section+'.'+name,'adwcode.自动重载');
      if (section+'.'+name===failAt) throw Error('模拟写入失败');
      if(value===undefined)values.delete(section+'.'+name);else values.set(section+'.'+name,value);
    },
  }}},
  window:{async showInformationMessage(message,options,...actions){messages.push({message,options,actions});return options?.modal?'应用':'重载窗口'},showErrorMessage(){errors++}},
  commands:{async executeCommand(){throw Error('不应执行真实窗口命令')}},
};
const context={globalState:{get(){return backup?JSON.parse(JSON.stringify(backup)):undefined},async update(_key,value){backup=value?JSON.parse(JSON.stringify(value)):undefined}}};
const sandbox={module:{exports:{}},process:{platform:'linux'},setTimeout,clearTimeout,require(name){
  if(name==='vscode')return vscode;
  if(name==='child_process')return {execFile(_file,args,_options,callback){callback(monoFont?null:Error('无 GNOME 设置'), args.includes('monospace-font-name')?monoFont||'':"'界面字体 11'")}};
  return require(name);
}};
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(path.join(__dirname,'../扩展/扩展.js'),'utf8'),sandbox);
(async()=>{
  failAt='editor.minimap.enabled';
  await sandbox.应用推荐设置(context);
  assert.equal(errors,1);
  assert.equal(values.get('adwcode.自动重载'),false);
  assert.equal(backup['adwcode.自动重载'].value,true);
  assert.equal(backup['editor.fontFamily'].value,'原有代码字体');
  assert.ok(!backup['editor.guides.indentation']);
  failAt=undefined;
  await sandbox.应用推荐设置(context);
  await sandbox.应用推荐设置(context);
  // 推荐主题必须在扩展中注册；操作者的工作区偏好可以独立覆盖推荐值。
  const contributes=JSON.parse(fs.readFileSync(path.join(__dirname,'../package.json'),'utf8')).contributes;
  for (const key of ['workbench.preferredLightColorTheme','workbench.preferredDarkColorTheme','workbench.preferredHighContrastLightColorTheme','workbench.preferredHighContrastColorTheme']) {
    assert.ok(contributes.themes.some(theme=>theme.label===values.get(key)),key);
  }
  assert.ok(contributes.productIconThemes.some(theme=>theme.id===values.get('workbench.productIconTheme')));
  assert.equal(values.get('window.titleBarStyle'),'custom');
  assert.equal(values.get('window.controlsStyle'),'native');
  assert.equal(values.get('window.menuBarVisibility'),'compact');
  assert.ok(!backup['python.defaultInterpreterPath']);
  assert.ok(!backup['mesonbuild.buildFolder']);
  assert.equal(backup['editor.fontFamily'].value,'原有代码字体');
  failAt='editor.minimap.enabled';
  await sandbox.恢复推荐设置(context);
  assert.equal(errors,2);
  assert.equal(values.get('editor.fontFamily'),'原有代码字体');
  assert.ok(!backup['editor.fontFamily']);
  assert.ok(backup['editor.minimap.enabled']);
  failAt=undefined;
  await sandbox.恢复推荐设置(context);
  assert.equal(backup,undefined);
  assert.equal(values.get('editor.fontFamily'),'原有代码字体');
  assert.equal(values.get('adwcode.自动重载'),true);
  assert.equal(values.get('workbench.productIconTheme'),'原有产品图标');
  assert.ok(!values.has('editor.minimap.enabled'));
  assert.ok(!messages.some(message=>message.options==='重载窗口'||message.actions.includes('重载窗口')));
  // 较旧的 VS Code 不提供某项时，预览和备份应跳过；工作区覆盖需准确提示。
  unavailable='window.controlsStyle';
  values.set(unavailable,'旧版保留值');
  workspaceReload=true;
  monoFont="'等距更纱黑体 SC 11'";
  await sandbox.应用推荐设置(context);
  assert.equal(values.get('editor.fontFamily'),'"等距更纱黑体 SC", "Adwaita Mono", monospace');
  assert.equal(values.get(unavailable),'旧版保留值');
  assert.ok(!backup[unavailable]);
  assert.ok(messages.some(message=>message.message.includes('已跳过：window.controlsStyle')));
  assert.ok(messages.some(message=>message.message.includes('工作区仍开启')));
  await sandbox.恢复推荐设置(context);
  workspaceReload=undefined;
  let release;
  vscode.window.showInformationMessage=()=>new Promise(resolve=>{release=resolve});
  const first=sandbox.应用推荐设置(context);
  await new Promise(resolve=>setImmediate(resolve));
  const before=writes;
  await sandbox.应用推荐设置(context);
  assert.equal(writes,before);
  release(undefined);
  await first;
  assert.equal(backup,undefined);
  console.log('推荐设置：外观对齐、重载关闭、字体、兼容、重复应用、恢复、并发与取消测试通过');
})().catch(error=>{console.error(error);process.exitCode=1});
