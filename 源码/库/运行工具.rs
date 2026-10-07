// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 隔离任务共用的程序查找、进程组清理与目录复制。

use crate::错误::结果;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// 在 PATH 中查找可执行文件。
#[must_use]
pub fn 查找程序(名称: &str) -> Option<PathBuf> {
    let 路径 = std::env::var_os("PATH")?;
    std::env::split_paths(&路径)
        .map(|目录| 目录.join(名称))
        .find(|候选| {
            候选.is_file()
                && 候选
                    .metadata()
                    .is_ok_and(|信息| 信息.permissions().mode() & 0o111 != 0)
        })
}

/// 子进程一启动即接管；错误返回也会清理其独立进程组。
pub(crate) struct 受管进程(pub(crate) Child);

impl Drop for 受管进程 {
    fn drop(&mut self) {
        终止进程组(self.0.id(), "-TERM");
        let 截止 = Instant::now() + Duration::from_secs(1);
        while Instant::now() < 截止 {
            if self.0.try_wait().ok().flatten().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // 主进程已退出也可能仍有后代持有调试端口。
        终止进程组(self.0.id(), "-KILL");
        let _ = self.0.wait();
    }
}

pub(crate) fn 复制目录(来源: &Path, 目标: &Path, 忽略: fn(&Path) -> bool) -> 结果<()> {
    crate::文件事务::校验普通路径(来源)?;
    crate::文件事务::校验普通路径(目标)?;
    std::fs::create_dir_all(目标)?;
    for 条目 in std::fs::read_dir(来源)? {
        let 条目 = 条目?;
        let 来源路径 = 条目.path();
        crate::文件事务::校验普通路径(&来源路径)?;
        if 忽略(&来源路径) {
            continue;
        }
        let 目标路径 = 目标.join(条目.file_name());
        crate::文件事务::校验普通路径(&目标路径)?;
        if 来源路径.is_dir() {
            复制目录(&来源路径, &目标路径, 忽略)?;
        } else if 来源路径.is_file() {
            std::fs::copy(&来源路径, &目标路径)?;
        }
    }
    Ok(())
}

fn 终止进程组(pid: u32, 信号: &str) {
    let _ = Command::new("kill")
        .arg(信号)
        .arg("--")
        .arg(format!("-{pid}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::错误::工具错误;
    use std::os::unix::process::CommandExt;

    #[test]
    fn 错误返回清理已启动的进程() {
        let 临时 = tempfile::tempdir().unwrap();
        let 标记 = 临时.path().join("不应写入");
        let 运行 = || -> 结果<()> {
            let _进程 = 受管进程(
                Command::new("sh")
                    .arg("-c")
                    .arg("sleep 1; touch \"$1\"")
                    .arg("sh")
                    .arg(&标记)
                    .process_group(0)
                    .spawn()?,
            );
            Err(工具错误::新("模拟后续步骤失败"))
        };
        assert!(运行().is_err());
        std::thread::sleep(Duration::from_millis(1200));
        assert!(!标记.exists());
    }

    #[test]
    fn 目录复制保留不同过滤策略并传播错误() {
        let 临时 = tempfile::tempdir().unwrap();
        let 来源 = 临时.path().join("来源");
        std::fs::create_dir_all(来源.join("__pycache__")).unwrap();
        std::fs::write(来源.join("__pycache__/样本"), "缓存").unwrap();
        std::fs::write(来源.join("普通"), "正文").unwrap();
        std::fs::write(来源.join("缓存.pyc"), "缓存").unwrap();
        let 全部 = 临时.path().join("全部");
        复制目录(&来源, &全部, |_| false).unwrap();
        assert!(全部.join("__pycache__/样本").exists());
        assert!(全部.join("缓存.pyc").exists());
        let 过滤 = 临时.path().join("过滤");
        复制目录(&来源, &过滤, |路径| {
            路径
                .components()
                .any(|部件| 部件.as_os_str() == "__pycache__")
                || 路径.extension().is_some_and(|扩展| 扩展 == "pyc")
        })
        .unwrap();
        assert_eq!(std::fs::read_to_string(过滤.join("普通")).unwrap(), "正文");
        assert!(!过滤.join("缓存.pyc").exists());
        assert!(!过滤.join("__pycache__").exists());
        assert!(复制目录(&来源.join("不存在"), &过滤, |_| false).is_err());
        assert!(复制目录(&来源, &来源.join("普通"), |_| false).is_err());
    }

    #[test]
    fn 复制拒绝来源目标及被过滤的符号链接() {
        use std::os::unix::fs::symlink;
        let 临时 = tempfile::tempdir().unwrap();
        let 来源 = 临时.path().join("来源");
        let 目标 = 临时.path().join("目标");
        std::fs::create_dir_all(&来源).unwrap();
        std::fs::create_dir_all(&目标).unwrap();
        symlink(&来源, 临时.path().join("来源链接")).unwrap();
        symlink(&目标, 临时.path().join("目标链接")).unwrap();
        assert!(复制目录(&临时.path().join("来源链接"), &目标, |_| false).is_err());
        assert!(复制目录(&来源, &临时.path().join("目标链接"), |_| false).is_err());
        symlink(&目标, 来源.join("缓存.pyc")).unwrap();
        assert!(复制目录(&来源, &目标, |_| true).is_err());
    }
}
