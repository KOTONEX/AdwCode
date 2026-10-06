## 变更内容

<!-- 简述这个 PR 做了什么、为什么。若关联 issue，请写 "Closes #123"。 -->

## 类型

- [ ] `新增` 新功能
- [ ] `修复` 缺陷修复
- [ ] `文档` 文档
- [ ] `测试` 测试
- [ ] `重构` 重构
- [ ] `杂务` 杂项
- [ ] `初始化` 初始提交

## 自测

- [ ] `meson compile -C builddir lint` 通过（含 Ruff 检查和格式检查）
- [ ] `meson compile -C builddir check` 通过
- [ ] `meson test -C builddir --print-errorlogs` 通过（未运行时请在下方说明原因）

## 约束检查

- [ ] 改动 `themes/` 后已重跑 `meson compile -C builddir themes`，且未手工编辑生成物
- [ ] 主题颜色使用十六进制（`#rrggbb` / `#rrggbbaa`），`contrastBorder` 只出现在高对比度主题
- [ ] 新增颜色键已存在于 `src/vscode_defaults/registry_keys.json` 或 `build.LEGACY_KEYS`
