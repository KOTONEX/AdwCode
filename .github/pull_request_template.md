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

- [ ] `cargo run --quiet -- 检查` 通过（含 rustfmt、clippy、Rust 与 JS 测试）
- [ ] `cargo run --quiet -- 校验` 通过
- [ ] `cargo test` 通过（未运行时请在下方说明原因）

## 约束检查

- [ ] 改动 `主题/` 后已重跑 `cargo run --quiet -- 主题`，且未手工编辑生成物
- [ ] 主题颜色使用十六进制（`#rrggbb` / `#rrggbbaa`），`contrastBorder` 只出现在高对比度主题
- [ ] 新增颜色键已存在于 `源码/VSCode默认数据/registry_keys.json` 或 `主题生成::补充颜色键`
