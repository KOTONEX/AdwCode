// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 保持插入顺序的字符串映射；主题颜色等输出依赖该顺序。

use indexmap::{IndexMap, map::Slice};

#[derive(Default, Clone, Debug)]
pub struct 有序映射 {
    条目: IndexMap<String, String>,
}

impl 有序映射 {
    #[must_use]
    pub fn 新() -> Self {
        Self::default()
    }

    pub fn 放(&mut self, 键: impl Into<String>, 值: impl Into<String>) {
        self.条目.insert(键.into(), 值.into());
    }

    #[must_use]
    pub fn 取(&self, 键: &str) -> Option<&str> {
        self.条目.get(键).map(String::as_str)
    }

    /// 内部角色缺失说明生成逻辑有误，与 Python 的 KeyError 一致直接失败。
    #[must_use]
    pub fn 必须取(&self, 键: &str) -> &str {
        match self.取(键) {
            Some(值) => 值,
            None => panic!("未知颜色角色： {键:?}"),
        }
    }

    /// 构造阶段读取中间结果时使用，返回所有权以免与写入冲突。
    #[must_use]
    pub fn 取拷贝(&self, 键: &str) -> String {
        self.必须取(键).to_string()
    }

    #[must_use]
    pub fn 条目(&self) -> &Slice<String, String> {
        self.条目.as_slice()
    }

    #[must_use]
    pub fn 包含(&self, 键: &str) -> bool {
        self.取(键).is_some()
    }

    #[must_use]
    pub fn 数量(&self) -> usize {
        self.条目.len()
    }
}

impl PartialEq for 有序映射 {
    fn eq(&self, 其他: &Self) -> bool {
        self.条目.as_slice() == 其他.条目.as_slice()
    }
}

impl Eq for 有序映射 {}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 保持插入顺序并可查找() {
        let mut 映射 = 有序映射::新();
        映射.放("第一", "#000000");
        映射.放("第二", "#ffffff");
        assert_eq!(映射.条目().first().unwrap().0, "第一");
        assert_eq!(映射.取("第二"), Some("#ffffff"));
        assert_eq!(映射.取("第三"), None);
        assert!(映射.包含("第一"));
    }

    #[test]
    fn 重复键原位更新而不增加条目() {
        let mut 映射 = 有序映射::新();
        映射.放("第一", "旧值");
        映射.放("第二", "第二值");
        映射.放("第一", "新值");
        assert_eq!(映射.取("第一"), Some("新值"));
        assert_eq!(映射.数量(), 2);
        assert_eq!(映射.条目().get_index(0).unwrap().0, "第一");
    }
    #[test]
    fn 相同内容的不同顺序仍不相等() {
        let mut 前 = 有序映射::新();
        前.放("第一", "一");
        前.放("第二", "二");
        let mut 后 = 有序映射::新();
        后.放("第二", "二");
        后.放("第一", "一");
        assert_ne!(前, 后);
        assert_eq!(前, 前.clone());
    }
}
