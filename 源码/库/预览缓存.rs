// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 提交预览的可删除 VSIX 缓存；运行会话保留独立副本。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{安装目标, 校验包, 运行};
use crate::摘要::sha256十六进制;
use crate::错误::{工具错误, 结果};

pub(super) fn 缓存目录() -> 结果<PathBuf> {
    let 基础 = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|路径| 路径.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|路径| 路径.is_absolute())
                .map(|路径| 路径.join(".cache"))
        })
        .ok_or_else(|| 工具错误::新("未设置绝对 XDG_CACHE_HOME 或 HOME"))?;
    Ok(基础.join("adwcode/预览"))
}

fn 可选读取(路径: &Path) -> 结果<Option<Vec<u8>>> {
    crate::文件事务::校验普通路径(路径)?;
    match std::fs::read(路径) {
        Ok(数据) => Ok(Some(数据)),
        Err(错误) if 错误.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(错误) => Err(错误.into()),
    }
}

fn 构建环境键(键: &str) -> bool {
    [
        "CARGO_",
        "RUST",
        "NODE_",
        "TSC_",
        "CC_",
        "CXX_",
        "CFLAGS_",
        "CXXFLAGS_",
        "AR_",
    ]
    .iter()
    .any(|前缀| 键.starts_with(前缀))
        || matches!(
            键,
            "HOME"
                | "CC"
                | "CXX"
                | "CFLAGS"
                | "CXXFLAGS"
                | "AR"
                | "LD"
                | "LDFLAGS"
                | "LD_LIBRARY_PATH"
                | "LD_PRELOAD"
                | "SOURCE_DATE_EPOCH"
                | "TZ"
                | "LANG"
                | "LC_ALL"
        )
}

fn 查找构建程序(
    副本: &Path, 搜索路径: &std::ffi::OsStr, 名称: &str
) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(搜索路径)
        .map(|目录| {
            if 目录.is_absolute() {
                目录
            } else {
                副本.join(目录)
            }
            .join(名称)
        })
        .find(|路径| {
            路径.is_file()
                && 路径
                    .metadata()
                    .is_ok_and(|信息| 信息.permissions().mode() & 0o111 != 0)
        })
}

// 标签与编辑器选项不影响包内容，必须在每次调用时重新判断，不能从缓存恢复目标环境。
pub(super) fn 构建输入(副本: &Path, 目标: &安装目标) -> 结果<Value> {
    let mut 工具链 = BTreeMap::new();
    for (程序, 参数) in [
        ("rustc", vec!["-vV"]),
        ("cargo", vec!["-Vv"]),
        ("node", vec!["--version"]),
        ("tsc", vec!["--version"]),
    ] {
        let 路径 = std::env::var_os("PATH")
            .and_then(|搜索路径| 查找构建程序(副本, &搜索路径, 程序))
            .ok_or_else(|| 工具错误::新(format!("未找到 {程序}")))?
            .canonicalize()?;
        let mut 信息 = json!({"版本":运行(副本, 程序, &参数, 30)?,
            "程序摘要":sha256十六进制(&std::fs::read(&路径)?)});
        // npm 的入口脚本会转交给真正的编译器；同时识别原生编译器入口。
        if 程序 == "tsc" {
            let 库 = 路径.parent().unwrap().join("../lib");
            for 文件 in ["_tsc.js", "tsc.js", "typescript.js"] {
                let 路径 = 库.join(文件);
                // 工具链安装中的符号链接由包管理器管理，不采用仓库路径的拒绝策略。
                match std::fs::read(&路径) {
                    Ok(数据) => 信息[文件] = json!(sha256十六进制(&数据)),
                    Err(错误) if 错误.kind() == std::io::ErrorKind::NotFound => {}
                    Err(错误) => return Err(错误.into()),
                }
            }
        }
        工具链.insert(程序, 信息);
    }
    let 环境 = std::env::vars_os()
        .filter(|(键, _)| 构建环境键(&键.to_string_lossy()))
        .map(|(键, 值)| (键, 值.as_encoded_bytes().to_vec()))
        .collect::<BTreeMap<_, _>>();
    let mut 配置 = BTreeMap::new();
    let mut 目录 = 副本
        .ancestors()
        .skip(1)
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    let 用户目录 = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|路径| PathBuf::from(路径).join(".cargo")));
    // 工作副本与其配置已有提交哈希覆盖；这里只读取会话祖先和 Cargo 用户配置。
    目录 = 目录.into_iter().map(|路径| 路径.join(".cargo")).collect();
    目录.extend(用户目录.map(|路径| {
        if 路径.is_absolute() {
            路径
        } else {
            副本.join(路径)
        }
    }));
    for 目录 in 目录 {
        for 名称 in ["config", "config.toml"] {
            let 路径 = 目录.join(名称);
            if let Some(数据) = 可选读取(&路径)? {
                配置.insert(路径, sha256十六进制(&数据));
            }
        }
    }
    // 环境只存摘要，避免将构建环境中的令牌等值写入元数据。
    let 环境 = 环境
        .into_iter()
        .map(|(键, 值)| (键.to_string_lossy().into_owned(), sha256十六进制(&值)))
        .collect::<BTreeMap<_, _>>();
    Ok(
        json!({"格式":1,"提交":目标.提交,"版本":目标.版本,"标识":目标.标识,
        "工具链":工具链,"环境":环境,"外部配置":配置,
        "预览执行器":sha256十六进制(&std::fs::read(std::env::current_exe()?)?)}),
    )
}

pub(super) struct 包缓存 {
    pub(super) 键: String,
    pub(super) 目录: PathBuf,
    输入: Value,
    // 同一键的读取、构建、写入串行；文件关闭后自动释放锁，不删除锁文件。
    _锁: std::fs::File,
}

impl 包缓存 {
    pub(super) fn 打开(基础: &Path, 输入: Value) -> 结果<Self> {
        let 键 = sha256十六进制(&serde_json::to_vec(&输入).unwrap());
        let 目录 = 基础.join(&键);
        let 锁路径 = 基础.join(format!("{键}.锁"));
        crate::文件事务::校验普通路径(&目录)?;
        crate::文件事务::校验普通路径(&锁路径)?;
        std::fs::create_dir_all(&目录)?;
        let 锁 = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(锁路径)?;
        锁.lock()?;
        Ok(Self {
            键,
            目录,
            输入,
            _锁: 锁,
        })
    }

    pub(super) fn 读取(&self, 目标: &安装目标) -> 结果<Option<Vec<u8>>> {
        let Some(记录) = 可选读取(&self.目录.join("校验.json"))? else {
            return Ok(None);
        };
        let Some(数据) = 可选读取(&self.目录.join("预览.vsix"))? else {
            return Ok(None);
        };
        let Ok(记录) = serde_json::from_slice::<Value>(&记录) else {
            return Ok(None);
        };
        if 记录["输入"] != self.输入
            || 记录["双次打包一致"] != true
            || 记录["SHA256"] != sha256十六进制(&数据)
            || 校验包(&数据, 目标).is_err()
        {
            return Ok(None);
        }
        Ok(Some(数据))
    }

    pub(super) fn 写入(&self, 数据: &[u8], 目标: &安装目标) -> 结果<()> {
        校验包(数据, 目标)?;
        let 记录 = json!({"输入":self.输入,"双次打包一致":true,"SHA256":sha256十六进制(数据)});
        crate::文件事务::写入批次(&[
            (self.目录.join("预览.vsix"), Some(数据.to_vec())),
            (
                self.目录.join("校验.json"),
                Some(crate::主题生成::写json(&记录).into_bytes()),
            ),
        ])
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::io::Write;

    fn 目标() -> 安装目标 {
        安装目标 {
            提交: "a".repeat(40),
            版本: "1.0.0".into(),
            标识: "测试.adwcode".into(),
            用户环境: false,
        }
    }

    fn 包(附加: &str, 版本: &str) -> Vec<u8> {
        let mut 输出 = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let 选项 = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        输出.start_file("extension/package.json", 选项).unwrap();
        write!(
            输出,
            "{}",
            json!({"publisher":"测试","name":"adwcode","version":版本})
        )
        .unwrap();
        输出.start_file(附加, 选项).unwrap();
        输出.write_all(b"CHECK_CRC_PAYLOAD").unwrap();
        输出.finish().unwrap().into_inner()
    }

    #[test]
    fn 缓存完整性失效和恢复() {
        let 临时 = tempfile::tempdir().unwrap();
        let 目标 = 目标();
        let 缓存 = 包缓存::打开(临时.path(), json!({"提交":目标.提交})).unwrap();
        assert!(缓存.读取(&目标).unwrap().is_none());
        let 数据 = 包("extension/正文", "1.0.0");
        缓存.写入(&数据, &目标).unwrap();
        assert_eq!(缓存.读取(&目标).unwrap().unwrap(), 数据);
        let 包路径 = 缓存.目录.join("预览.vsix");
        let 记录路径 = 缓存.目录.join("校验.json");
        let 原记录 = std::fs::read(&记录路径).unwrap();
        for 损坏记录 in [b"{".to_vec(), b"null".to_vec(), b"{}".to_vec()] {
            std::fs::write(&记录路径, 损坏记录).unwrap();
            assert!(缓存.读取(&目标).unwrap().is_none());
        }
        std::fs::write(&记录路径, &原记录).unwrap();
        std::fs::write(&包路径, b"not a zip").unwrap();
        assert!(缓存.读取(&目标).unwrap().is_none());
        // 摘要一致仍须检查 ZIP 的 CRC，不能仅信任元数据。
        let mut 损坏 = 数据.clone();
        let 偏移 = 损坏
            .windows(b"CHECK_CRC_PAYLOAD".len())
            .position(|段| 段 == b"CHECK_CRC_PAYLOAD")
            .unwrap();
        损坏[偏移] ^= 1;
        std::fs::write(&包路径, &损坏).unwrap();
        let mut 记录: Value = serde_json::from_slice(&原记录).unwrap();
        记录["SHA256"] = json!(sha256十六进制(&损坏));
        std::fs::write(&记录路径, serde_json::to_vec(&记录).unwrap()).unwrap();
        assert!(缓存.读取(&目标).unwrap().is_none());
        缓存.写入(&数据, &目标).unwrap();
        assert_eq!(缓存.读取(&目标).unwrap().unwrap(), 数据);
        // 无效新包不得替换已通过验收的旧缓存。
        for 无效 in [
            包("extension/正文", "2.0.0"),
            包("extension/.vscode/settings.json", "1.0.0"),
        ] {
            assert!(缓存.写入(&无效, &目标).is_err());
            assert_eq!(缓存.读取(&目标).unwrap().unwrap(), 数据);
        }
        std::fs::remove_file(记录路径).unwrap();
        assert!(缓存.读取(&目标).unwrap().is_none());
    }

    #[test]
    fn 缓存拒绝符号链接并传播读取故障() {
        use std::os::unix::fs::symlink;
        let 临时 = tempfile::tempdir().unwrap();
        let 缓存 = 包缓存::打开(临时.path(), json!({})).unwrap();
        let 外部 = 临时.path().join("外部");
        std::fs::write(&外部, "原值").unwrap();
        let 记录 = 缓存.目录.join("校验.json");
        symlink(&外部, &记录).unwrap();
        assert!(缓存.读取(&目标()).is_err());
        assert!(缓存.写入(&包("extension/正文", "1.0.0"), &目标()).is_err());
        assert_eq!(std::fs::read_to_string(&外部).unwrap(), "原值");
        std::fs::remove_file(&记录).unwrap();
        std::fs::create_dir(&记录).unwrap();
        assert!(缓存.读取(&目标()).is_err());
        let 链接 = 临时.path().join("链接");
        symlink(临时.path(), &链接).unwrap();
        assert!(包缓存::打开(&链接, json!({})).is_err());
    }

    #[test]
    fn 缓存键随构建条件变化且锁在关闭后释放() {
        let 临时 = tempfile::tempdir().unwrap();
        let 输入 = json!({"提交":"a", "工具链":"1", "环境":"1", "外部配置":"1", "预览执行器":"1"});
        let 缓存 = 包缓存::打开(临时.path(), 输入.clone()).unwrap();
        let 原键 = 缓存.键.clone();
        let 锁 = std::fs::File::options()
            .read(true)
            .write(true)
            .open(临时.path().join(format!("{原键}.锁")))
            .unwrap();
        assert!(锁.try_lock().is_err());
        drop(缓存);
        assert!(锁.try_lock().is_ok());
        drop(锁);
        assert_eq!(包缓存::打开(临时.path(), 输入.clone()).unwrap().键, 原键);
        for 字段 in ["提交", "工具链", "环境", "外部配置", "预览执行器"] {
            let mut 变更 = 输入.clone();
            变更[字段] = json!("已改变");
            assert_ne!(包缓存::打开(临时.path(), 变更).unwrap().键, 原键);
        }
    }

    #[test]
    fn 输入不含随机会话路径或用户环境判断() {
        let 临时 = tempfile::tempdir().unwrap();
        let 副本甲 = 临时.path().join("甲/源码");
        let 副本乙 = 临时.path().join("乙/源码");
        std::fs::create_dir_all(&副本甲).unwrap();
        std::fs::create_dir_all(&副本乙).unwrap();
        let mut 目标 = 目标();
        let 甲 = 构建输入(&副本甲, &目标).unwrap();
        目标.用户环境 = true;
        assert_eq!(甲, 构建输入(&副本乙, &目标).unwrap());
        // 会话祖先的 Cargo 配置是实际构建输入，变化后必须失效。
        let 配置 = 临时.path().join(".cargo/config.toml");
        std::fs::create_dir_all(配置.parent().unwrap()).unwrap();
        std::fs::write(&配置, "[build]\njobs = 2\n").unwrap();
        assert_ne!(甲, 构建输入(&副本乙, &目标).unwrap());
        assert!(构建环境键("RUSTFLAGS"));
        assert!(构建环境键("CARGO_PROFILE_RELEASE_LTO"));
        assert!(!构建环境键("VSCODE_IPC_HOOK_CLI"));
    }

    #[test]
    fn 相对工具路径按实际构建目录解析() {
        use std::os::unix::fs::PermissionsExt;
        let 临时 = tempfile::tempdir().unwrap();
        let 副本 = 临时.path().join("源码");
        let 路径 = 副本.join("工具/node");
        std::fs::create_dir_all(路径.parent().unwrap()).unwrap();
        std::fs::write(&路径, "").unwrap();
        assert!(查找构建程序(&副本, std::ffi::OsStr::new("工具"), "node").is_none());
        std::fs::set_permissions(&路径, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            查找构建程序(&副本, std::ffi::OsStr::new("工具"), "node").unwrap(),
            路径
        );
        assert_eq!(
            查找构建程序(路径.parent().unwrap(), std::ffi::OsStr::new(""), "node").unwrap(),
            路径
        );
    }
}
