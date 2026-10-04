// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode contributors
// 使用内存配置与命令模拟，验证重复应用和部分失败，不访问工作窗口。
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const values = new Map([['editor.fontFamily','原有代码字体']]);
let backup, failAt, errors=0, writes=0;
const vscode = {
  ConfigurationTarget: {Global:1},
  workspace: {getConfiguration(section){return {
    get(name,fallback){return values.has(section+'.'+name)?values.get(section+'.'+name):fallback},
    inspect(name){return {globalValue:values.get(section+'.'+name)}},
    async update(name,value){
      assert.ok(backup, '配置写入之前必须保留备份');
      writes++;
      if (section+'.'+name===failAt) throw Error('模拟写入失败');
      if(value===undefined)values.delete(section+'.'+name);else values.set(section+'.'+name,value);
    },
  }}},
  window:{async showInformationMessage(_message,options){return options?.modal?'应用':undefined},showErrorMessage(){errors++}},
  commands:{async executeCommand(){throw Error('不应执行真实窗口命令')}},
};
const context={globalState:{get(){return backup?JSON.parse(JSON.stringify(backup)):undefined},async update(_key,value){backup=value?JSON.parse(JSON.stringify(value)):undefined}}};
const sandbox={module:{exports:{}},process:{platform:'linux'},setTimeout,clearTimeout,require(name){
  if(name==='vscode')return vscode;
  if(name==='child_process')return {execFile(_file,_args,_options,callback){callback(Error('无 GNOME 设置'), '')}};
  return require(name);
}};
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(path.join(__dirname,'../extension/extension.js'),'utf8'),sandbox);
(async()=>{
  failAt='editor.minimap.enabled';
  await sandbox.applyRecommendedSettings(context);
  assert.equal(errors,1);
  assert.equal(backup['editor.fontFamily'].value,'原有代码字体');
  assert.ok(!backup['editor.guides.indentation']);
  failAt=undefined;
  await sandbox.applyRecommendedSettings(context);
  await sandbox.applyRecommendedSettings(context);
  assert.equal(backup['editor.fontFamily'].value,'原有代码字体');
  failAt='editor.minimap.enabled';
  await sandbox.revertRecommendedSettings(context);
  assert.equal(errors,2);
  assert.equal(values.get('editor.fontFamily'),'原有代码字体');
  assert.ok(!backup['editor.fontFamily']);
  assert.ok(backup['editor.minimap.enabled']);
  failAt=undefined;
  await sandbox.revertRecommendedSettings(context);
  assert.equal(backup,undefined);
  assert.equal(values.get('editor.fontFamily'),'原有代码字体');
  assert.ok(!values.has('editor.minimap.enabled'));
  let release;
  vscode.window.showInformationMessage=()=>new Promise(resolve=>{release=resolve});
  const first=sandbox.applyRecommendedSettings(context);
  await new Promise(resolve=>setImmediate(resolve));
  const before=writes;
  await sandbox.applyRecommendedSettings(context);
  assert.equal(writes,before);
  release(undefined);
  await first;
  assert.equal(backup,undefined);
  console.log('推荐设置：重复应用、部分失败、逐项恢复、并发与取消测试通过');
})().catch(error=>{console.error(error);process.exitCode=1});
