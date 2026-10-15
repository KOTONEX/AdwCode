// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 从交付提交构建 VSIX；无标签时隔离安装，有匹配版本标签时安装到日常环境。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::错误::{工具错误, 结果};

#[path = "预览缓存.rs"]
mod 缓存;

fn 运行(目录: &Path, 程序: &str, 参数: &[&str], 秒数: u64) -> 结果<String> {
    // 子工具按隔离副本定位根目录，不继承操作者用于当前工作副本的覆盖值。
    let mut 子参数 = vec!["-u".to_owned(), "ADWCODE根目录".to_owned(), 程序.to_owned()];
    子参数.extend(参数.iter().map(|值| (*值).to_owned()));
    let 输出 =
        crate::发布执行::运行(目录, 程序, "env", &子参数, Duration::from_secs(秒数), &[])?;
    if !输出.成功 {
        return Err(工具错误::新(format!(
            "{程序} 执行失败：{}{}",
            输出.标准输出, 输出.诊断
        )));
    }
    Ok(输出.标准输出)
}

struct 安装目标 {
    提交: String,
    版本: String,
    标识: String,
    用户环境: bool,
}

fn 读取目标(根目录: &Path) -> 结果<安装目标> {
    let 提交 = if 根目录.join(".jj").is_dir() {
        运行(
            根目录,
            "jj",
            &["log", "-r", "@-", "--no-graph", "-T", "commit_id"],
            30,
        )?
    } else {
        运行(
            根目录,
            "git",
            &["rev-parse", "--verify", "HEAD^{commit}"],
            30,
        )?
    };
    let 提交 = 提交.trim().to_owned();
    if !matches!(提交.len(), 40 | 64) || !提交.bytes().all(|字节| 字节.is_ascii_hexdigit())
    {
        return Err(工具错误::新("无法确定最新交付提交"));
    }
    let 清单 = 运行(
        根目录,
        "git",
        &["show", &format!("{提交}:package.json")],
        30,
    )?;
    let 清单: Value = serde_json::from_str(&清单)
        .map_err(|错误| 工具错误::新(format!("提交清单无法解析：{错误}")))?;
    let 读取字段 = |字段: &str| -> 结果<String> {
        清单[字段]
            .as_str()
            .filter(|值| !值.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| 工具错误::新(format!("提交清单缺少 {字段}")))
    };
    let 版本 = 读取字段("version")?;
    semver::Version::parse(&版本)
        .map_err(|错误| 工具错误::新(format!("无效扩展版本：{错误}")))?;
    let 标识 = format!("{}.{}", 读取字段("publisher")?, 读取字段("name")?);
    // --points-at 同时处理轻量与附注标签，不使用祖先提交上的版本标签。
    let 标签 = 运行(根目录, "git", &["tag", "--points-at", &提交], 30)?;
    let 用户环境 = 标签.lines().any(|标签| 标签 == format!("v{版本}"));
    Ok(安装目标 {
        提交,
        版本,
        标识,
        用户环境,
    })
}

fn 编辑器命令(程序: &Path, 隔离目录: Option<&Path>) -> Command {
    let mut 命令 = Command::new(程序);
    命令
        .env_remove("VSCODE_IPC_HOOK_CLI")
        .env_remove("VSCODE_CLI_REMOTE_AUTHORITY");
    if let Some(目录) = 隔离目录 {
        命令
            .env_remove("VSCODE_PORTABLE")
            .arg("--user-data-dir")
            .arg(目录.join("用户数据"))
            .arg("--extensions-dir")
            .arg(目录.join("扩展"));
    }
    命令
}

// Linux 便携版的 data 会覆盖 CLI 隔离参数；先拒绝，避免误安装到便携用户环境。
fn 校验隔离安装(程序: &Path) -> 结果<()> {
    let 程序 = 程序.canonicalize()?;
    let Some(目录) = 程序.parent() else {
        return Err(工具错误::新("编辑器路径没有父目录"));
    };
    let 安装目录 = if 目录.file_name().is_some_and(|名称| 名称 == "bin") {
        目录.parent().unwrap_or(目录)
    } else {
        目录
    };
    if 安装目录.join("resources/app/product.json").is_file()
        && 安装目录.join("data").try_exists()?
    {
        return Err(工具错误::新(
            "便携版 data 目录会覆盖隔离参数；无版本标签的预览请通过 --编辑器程序 选择非便携版 VS Code",
        ));
    }
    Ok(())
}

// VS Code CLI 自行管理 GUI 生命周期；不能用会清理整个进程组的发布执行器。
fn 执行编辑器(mut 命令: Command, 目录: &Path) -> 结果<String> {
    let mut 标准输出 = tempfile::tempfile()?;
    let mut 标准错误 = tempfile::tempfile()?;
    let mut 子进程 = 命令
        .current_dir(目录)
        .stdin(Stdio::null())
        .stdout(标准输出.try_clone()?)
        .stderr(标准错误.try_clone()?)
        .spawn()?;
    let 开始 = Instant::now();
    let 状态 = loop {
        if let Some(状态) = 子进程.try_wait()? {
            break 状态;
        }
        if 开始.elapsed() > Duration::from_secs(120) {
            let _ = 子进程.kill();
            let _ = 子进程.wait();
            return Err(工具错误::新(
                "VS Code CLI 超时；窗口状态尚未确认，预览目录会保留",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    use std::io::{Read, Seek};
    let mut 输出 = String::new();
    标准输出.rewind()?;
    标准输出.take(1024 * 1024).read_to_string(&mut 输出)?;
    if !状态.success() {
        标准错误.rewind()?;
        标准错误.take(65536).read_to_string(&mut 输出)?;
        return Err(工具错误::新(format!("VS Code CLI 失败：{输出}")));
    }
    Ok(输出)
}

fn 安装并打开(程序: &Path, 会话: &Path, 包: &Path, 目标: &安装目标) -> 结果<()> {
    let 隔离目录 = (!目标.用户环境).then_some(会话);
    let mut 安装 = 编辑器命令(程序, 隔离目录);
    安装.arg("--install-extension").arg(包).arg("--force");
    print!("{}", 执行编辑器(安装, 会话)?);
    let mut 核对 = 编辑器命令(程序, 隔离目录);
    核对.args(["--list-extensions", "--show-versions"]);
    let 已安装 = 执行编辑器(核对, 会话)?;
    if !已安装
        .lines()
        .any(|行| 行.eq_ignore_ascii_case(&format!("{}@{}", 目标.标识, 目标.版本)))
    {
        return Err(工具错误::新(
            "VS Code 未报告目标扩展版本，停止打开窗口",
        ));
    }
    let mut 打开 = 编辑器命令(程序, 隔离目录);
    if 隔离目录.is_some() {
        // 全新配置不信任源码副本，会禁用本扩展；只对独立预览会话关闭信任检查。
        打开.arg("--disable-workspace-trust");
    }
    打开
        .args(["--new-window", "--skip-add-to-recently-opened"])
        .arg(会话.join("源码"));
    执行编辑器(打开, 会话)?;
    Ok(())
}

// application 范围设置不能通过扩展默认值注册；隔离会话须在首次启动前显式写入。
// 只读取已校验 VSIX 的清单，旧提交仍可从 configurationDefaults 获取这些设置。
fn 初始化隔离设置(
    会话: &Path, 清单: &Value, 用户环境: bool
) -> 结果<Option<Value>> {
    if 用户环境 {
        return Ok(None);
    }
    let mut 设置 = 清单
        .pointer("/contributes/configurationDefaults")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| 工具错误::新("VSIX 清单缺少有效的 configurationDefaults"))?;
    if let Some(推荐) = 清单.get("推荐设置") {
        let 推荐 = 推荐
            .as_object()
            .ok_or_else(|| 工具错误::新("VSIX 清单的推荐设置必须为对象"))?;
        for (键, 值) in 推荐 {
            设置.insert(键.clone(), 值.clone());
        }
    }
    let 设置 = Value::Object(设置);
    let 路径 = 会话.join("用户数据/User/settings.json");
    crate::文件事务::校验普通路径(&路径)?;
    std::fs::create_dir_all(路径.parent().unwrap())?;
    // 会话应当全新；拒绝覆盖已经存在的配置。
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(路径)?
        .write_all(crate::主题生成::写json(&设置).as_bytes())?;
    Ok(Some(设置))
}

fn 状态目录() -> 结果<PathBuf> {
    let 基础 = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|路径| 路径.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|路径| 路径.is_absolute())
                .map(|路径| 路径.join(".local/state"))
        })
        .ok_or_else(|| 工具错误::新("未设置绝对 XDG_STATE_HOME 或 HOME"))?;
    let 目录 = 基础.join("adwcode/安装预览");
    crate::文件事务::校验普通路径(&目录)?;
    std::fs::create_dir_all(&目录)?;
    Ok(目录)
}

fn 构建包(
    根目录: &Path, 副本: &Path, 会话: &Path, 目标: &安装目标
) -> 结果<Vec<u8>> {
    // 由目标提交自身的工具生成与打包，不混入当前未提交的构建逻辑。
    let 构建目录 = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|路径| {
            if 路径.is_absolute() {
                路径
            } else {
                根目录.join(路径)
            }
        })
        .unwrap_or_else(|| 根目录.join("target"));
    let 构建目录 = std::path::absolute(构建目录)?;
    运行(
        副本,
        "cargo",
        &[
            "build",
            "--release",
            "--locked",
            "--quiet",
            "--target-dir",
            构建目录
                .to_str()
                .ok_or_else(|| 工具错误::新("构建路径必须是 UTF-8"))?,
        ],
        600,
    )?;
    let 工具 = 构建目录.join("release/adwcode");
    let 工具 = 工具
        .to_str()
        .ok_or_else(|| 工具错误::新("构建路径必须是 UTF-8"))?;
    for 子命令 in ["主题", "生成自有字形", "生成导入字形"] {
        运行(副本, 工具, &[子命令], 120)?;
    }
    运行(
        副本,
        "git",
        &[
            "diff",
            "--exit-code",
            "--",
            "主题",
            "产品图标",
            "package.json",
        ],
        30,
    )?;
    // 只记录目标身份，不生成历史日志；也兼容仍默认读取历史的旧提交工具。
    let 日志路径 = 会话.join("预览摘要.md");
    std::fs::write(
        &日志路径,
        format!(
            "# 开发预览\n\n版本：{}\n\n提交：{}\n\n此包为提交预览，未附带完整变更历史。\n",
            目标.版本, 目标.提交
        ),
    )?;
    let 打包参数 = [
        "打包",
        "--变更日志",
        日志路径
            .to_str()
            .ok_or_else(|| 工具错误::新("预览路径必须是 UTF-8"))?,
    ];
    let 包 = 副本.join(format!("AdwCode-{}.vsix", 目标.版本));
    运行(副本, 工具, &打包参数, 120)?;
    let 数据 = std::fs::read(&包)?;
    运行(副本, 工具, &打包参数, 120)?;
    if 数据 != std::fs::read(&包)? {
        return Err(工具错误::新("两次 VSIX 打包字节不一致"));
    }
    Ok(数据)
}

fn 校验包(数据: &[u8], 目标: &安装目标) -> 结果<Value> {
    let mut 归档 = zip::ZipArchive::new(std::io::Cursor::new(&数据))
        .map_err(|错误| 工具错误::新(format!("VSIX 无法打开：{错误}")))?;
    let mut 清单 = String::new();
    use std::io::Read;
    归档
        .by_name("extension/package.json")
        .map_err(|错误| 工具错误::新(format!("VSIX 缺少清单：{错误}")))?
        .read_to_string(&mut 清单)?;
    let 清单: Value =
        serde_json::from_str(&清单).map_err(|错误| 工具错误::新(错误.to_string()))?;
    if 清单["version"] != 目标.版本
        || format!(
            "{}.{}",
            清单["publisher"].as_str().unwrap_or(""),
            清单["name"].as_str().unwrap_or("")
        ) != 目标.标识
    {
        return Err(工具错误::新("VSIX 清单与目标提交不一致"));
    }
    for 索引 in 0..归档.len() {
        let mut 文件 = 归档
            .by_index(索引)
            .map_err(|错误| 工具错误::新(format!("VSIX 条目损坏：{错误}")))?;
        if ["/.git/", "/.jj/", "/.vscode/", "firebase-debug.log"]
            .iter()
            .any(|标记| 文件.name().contains(标记))
        {
            return Err(工具错误::新("VSIX 含私有文件，停止安装"));
        }
        std::io::copy(&mut 文件, &mut std::io::sink())?;
    }
    Ok(清单)
}

fn 解析参数(参数: &[String]) -> 结果<(Option<PathBuf>, bool)> {
    let mut 编辑器 = None;
    let mut 重新构建 = false;
    let mut 参数 = 参数.iter();
    let 用法 = || 工具错误::新("用法：cargo 预览 [--重新构建] [--编辑器程序 路径]");
    while let Some(项) = 参数.next() {
        match 项.as_str() {
            "--重新构建" if !重新构建 => 重新构建 = true,
            "--编辑器程序" if 编辑器.is_none() => {
                let 路径 = 参数
                    .next()
                    .filter(|值| !值.starts_with("--"))
                    .ok_or_else(用法)?;
                编辑器 = Some(std::path::absolute(路径)?);
            }
            _ => return Err(用法()),
        }
    }
    Ok((编辑器, 重新构建))
}

/// `cargo 预览`：始终打开新窗口；只有精确匹配的版本标签允许安装到用户环境。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let (编辑器, 重新构建) = 解析参数(参数)?;
    let 程序 = 编辑器.map(Ok).unwrap_or_else(|| {
        crate::运行工具::查找程序("code").ok_or_else(|| 工具错误::新("未找到 code 命令"))
    })?;
    let 目标 = 读取目标(根目录)?;
    if !目标.用户环境 {
        校验隔离安装(&程序)?;
    }
    let 会话 = tempfile::Builder::new()
        .prefix("会话-")
        .tempdir_in(状态目录()?)?;
    let 副本 = 会话.path().join("源码");
    let 副本参数 = 副本
        .to_str()
        .ok_or_else(|| 工具错误::新("预览路径必须是 UTF-8"))?;
    let 来源 = 根目录
        .to_str()
        .ok_or_else(|| 工具错误::新("仓库路径必须是 UTF-8"))?;
    println!(
        "安装提交 {}，版本 {}，目标：{}",
        目标.提交,
        目标.版本,
        if 目标.用户环境 {
            "日常用户环境（匹配版本标签）"
        } else {
            "全新隔离环境"
        }
    );
    运行(
        根目录,
        "git",
        &[
            "clone",
            "--quiet",
            "--no-hardlinks",
            "--no-checkout",
            来源,
            副本参数,
        ],
        120,
    )?;
    运行(
        &副本,
        "git",
        &["checkout", "--quiet", "--detach", &目标.提交],
        30,
    )?;
    let 缓存 = 缓存::包缓存::打开(&缓存::缓存目录()?, 缓存::构建输入(&副本, &目标)?)?;
    let 已缓存 = if 重新构建 {
        None
    } else {
        缓存.读取(&目标)?
    };
    let 命中 = 已缓存.is_some();
    let 数据 = if let Some(数据) = 已缓存 {
        println!("预览缓存命中，跳过构建、生成与打包：{}", 缓存.键);
        数据
    } else {
        println!(
            "{}，重新构建并校验双次打包",
            if 重新构建 {
                "已要求重新构建"
            } else {
                "预览缓存未命中或已损坏"
            }
        );
        let 数据 = 构建包(根目录, &副本, 会话.path(), &目标)?;
        缓存.写入(&数据, &目标)?;
        数据
    };
    let 缓存报告 = json!({"命中":命中,"强制重建":重新构建,"键":缓存.键,"目录":缓存.目录});
    drop(缓存);
    // 每个窗口持有独立字节副本，删除缓存或后续重建不会影响运行中的会话。
    let 包 = 副本.join(format!("AdwCode-{}.vsix", 目标.版本));
    std::fs::write(&包, &数据)?;
    let 清单 = 校验包(&数据, &目标)?;
    let 预览设置 = 初始化隔离设置(会话.path(), &清单, 目标.用户环境)?;
    // 从这一刻起保留会话，即使 CLI 错误也不删除可能已打开窗口的配置或源码。
    let 会话 = 会话.keep();
    let 报告路径 = 会话.join("验收.json");
    let mut 报告 = json!({"提交":目标.提交,"版本":目标.版本,"用户环境":目标.用户环境,
        "会话":会话,"包":包,"SHA256":crate::摘要::sha256十六进制(&数据),
        "隔离推荐设置":预览设置,"缓存":缓存报告,"完成":false});
    std::fs::write(&报告路径, crate::主题生成::写json(&报告))?;
    安装并打开(&程序, &会话, &包, &目标).map_err(|错误| {
        工具错误::新(format!("{错误}；会话与报告已保留：{}", 报告路径.display()))
    })?;
    报告["完成"] = json!(true);
    // CLI 成功表示已接受新窗口请求，不冒充视觉或扩展宿主验收。
    报告["窗口请求已接受"] = json!(true);
    std::fs::write(&报告路径, crate::主题生成::写json(&报告))?;
    println!("已安装并请求打开新窗口。报告：{}", 报告路径.display());
    println!("窗口关闭后可删除整个会话目录：{}", 会话.display());
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn git(目录: &Path, 参数: &[&str]) -> String {
        运行(目录, "git", 参数, 10).unwrap()
    }

    #[test]
    fn 只有目标提交的匹配版本标签允许用户环境() {
        let 临时 = tempfile::tempdir().unwrap();
        let 目录 = 临时.path();
        git(目录, &["init", "--quiet"]);
        git(目录, &["config", "user.name", "安装测试"]);
        git(目录, &["config", "user.email", "test@example.invalid"]);
        git(目录, &["config", "commit.gpgsign", "false"]);
        git(目录, &["config", "tag.gpgsign", "false"]);
        std::fs::write(
            目录.join("package.json"),
            r#"{"name":"adwcode","publisher":"测试","version":"1.0.0"}"#,
        )
        .unwrap();
        git(目录, &["add", "package.json"]);
        git(目录, &["commit", "--quiet", "-m", "测试清单"]);
        assert!(!读取目标(目录).unwrap().用户环境);
        git(目录, &["tag", "v1.0.0"]);
        assert!(读取目标(目录).unwrap().用户环境);
        git(目录, &["tag", "-d", "v1.0.0"]);
        git(目录, &["tag", "-a", "v1.0.0", "-m", "附注版本"]);
        assert!(读取目标(目录).unwrap().用户环境);
        // 祖先上的匹配版本标签也不允许写入用户环境。
        git(
            目录,
            &["commit", "--quiet", "--allow-empty", "-m", "后继提交"],
        );
        assert!(!读取目标(目录).unwrap().用户环境);
        git(目录, &["tag", "v2.0.0"]);
        git(目录, &["tag", "其他标签"]);
        assert!(!读取目标(目录).unwrap().用户环境);
        // 未提交的清单恰好与标签匹配也不能改变判断。
        std::fs::write(
            目录.join("package.json"),
            r#"{"name":"adwcode","publisher":"测试","version":"2.0.0"}"#,
        )
        .unwrap();
        let 目标 = 读取目标(目录).unwrap();
        assert_eq!(目标.版本, "1.0.0");
        assert!(!目标.用户环境);
    }

    #[test]
    fn 安装严格解析参数() {
        for 参数 in [
            vec!["--编辑器程序"],
            vec!["--不存在"],
            vec!["code", "多余"],
            vec!["--重新构建", "--重新构建"],
            vec!["--编辑器程序", "--重新构建"],
            vec!["--编辑器程序", "code", "--编辑器程序", "code"],
        ] {
            let 参数 = 参数.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(
                入口(Path::new("/不存在"), &参数)
                    .unwrap_err()
                    .消息
                    .contains("用法")
            );
        }
        for 参数 in [
            vec!["--重新构建", "--编辑器程序", "/编辑器"],
            vec!["--编辑器程序", "/编辑器", "--重新构建"],
        ] {
            let 参数 = 参数.into_iter().map(str::to_owned).collect::<Vec<_>>();
            let (程序, 重建) = 解析参数(&参数).unwrap();
            assert_eq!(程序.unwrap(), Path::new("/编辑器"));
            assert!(重建);
        }
    }

    #[test]
    fn 隔离配置来自包且用户环境不写入() {
        let 临时 = tempfile::tempdir().unwrap();
        let 会话 = 临时.path();
        let 旧清单 = json!({"contributes":{"configurationDefaults":{
            "window.titleBarStyle":"custom", "editor.minimap.enabled":false
        }}});
        // 标签模式不能创建配置目录，甚至不应解析预览专用字段。
        assert!(初始化隔离设置(会话, &Value::Null, true).unwrap().is_none());
        assert!(!会话.join("用户数据").exists());
        let 设置 = 初始化隔离设置(会话, &旧清单, false).unwrap().unwrap();
        assert_eq!(设置["window.titleBarStyle"], "custom");
        assert_eq!(设置["editor.minimap.enabled"], false);
        let 路径 = 会话.join("用户数据/User/settings.json");
        let 原值 = std::fs::read(&路径).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&原值).unwrap(), 设置);
        assert!(初始化隔离设置(会话, &旧清单, false).is_err());
        assert_eq!(std::fs::read(&路径).unwrap(), 原值);

        let 另一会话 = 会话.join("新会话");
        let 新清单 = json!({"contributes":{"configurationDefaults":{
            "editor.minimap.enabled":false
        }},"推荐设置":{"window.controlsStyle":"native"}});
        let 设置 = 初始化隔离设置(&另一会话, &新清单, false).unwrap().unwrap();
        assert_eq!(设置["window.controlsStyle"], "native");
        assert_eq!(设置["editor.minimap.enabled"], false);
        for 清单 in [
            Value::Null,
            json!({"contributes":{"configurationDefaults":{}},"推荐设置":[]}),
        ] {
            assert!(初始化隔离设置(&会话.join("无效"), &清单, false).is_err());
        }
        assert!(!会话.join("无效").exists());
    }

    #[test]
    fn 便携安装不能绕过无标签提交的隔离() {
        let 临时 = tempfile::tempdir().unwrap();
        let 根 = 临时.path();
        std::fs::create_dir_all(根.join("bin")).unwrap();
        std::fs::create_dir_all(根.join("resources/app")).unwrap();
        std::fs::write(根.join("resources/app/product.json"), "{}").unwrap();
        let 程序 = 根.join("bin/code");
        std::fs::write(&程序, "").unwrap();
        assert!(校验隔离安装(&程序).is_ok());
        std::fs::create_dir(根.join("data")).unwrap();
        assert!(校验隔离安装(&程序).unwrap_err().消息.contains("便携版"));
    }

    #[test]
    fn 编辑器环境不继承现有窗口连接且隔离目录独立() {
        let 命令 = 编辑器命令(Path::new("code"), Some(Path::new("/预览")));
        let 环境 = 命令
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        for 键 in [
            "VSCODE_IPC_HOOK_CLI",
            "VSCODE_CLI_REMOTE_AUTHORITY",
            "VSCODE_PORTABLE",
        ] {
            assert_eq!(环境.get(std::ffi::OsStr::new(键)), Some(&None));
        }
        assert_eq!(
            命令.get_args().collect::<Vec<_>>(),
            [
                "--user-data-dir",
                "/预览/用户数据",
                "--extensions-dir",
                "/预览/扩展"
            ]
        );
        let 命令 = 编辑器命令(Path::new("code"), None);
        assert_eq!(命令.get_args().count(), 0);
    }

    #[test]
    fn 安装失败或版本不匹配时不打开窗口() {
        use std::os::unix::fs::PermissionsExt;
        let 临时 = tempfile::tempdir().unwrap();
        let 会话 = 临时.path();
        let 程序 = 会话.join("模拟编辑器");
        std::fs::write(
            &程序,
            r#"#!/bin/sh
printf '%s\n' "$@" >> 参数
case " $* " in
  *' --install-extension '*) test ! -f 安装失败 ;;
  *' --list-extensions '*) if test ! -f 版本错误; then echo '测试.adwcode@1.0.0'; fi ;;
  *' --new-window '*) touch 已打开 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(&程序, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut 目标 = 安装目标 {
            提交: String::new(),
            版本: "1.0.0".into(),
            标识: "测试.adwcode".into(),
            用户环境: false,
        };
        let 包 = 会话.join("带 空格.vsix");
        for 标记 in ["安装失败", "版本错误"] {
            std::fs::write(会话.join(标记), "").unwrap();
            assert!(安装并打开(&程序, 会话, &包, &目标).is_err());
            assert!(!会话.join("已打开").exists());
            std::fs::remove_file(会话.join(标记)).unwrap();
        }
        std::fs::write(会话.join("参数"), "").unwrap();
        安装并打开(&程序, 会话, &包, &目标).unwrap();
        assert!(会话.join("已打开").exists());
        let 参数 = std::fs::read_to_string(会话.join("参数")).unwrap();
        assert_eq!(
            参数.lines().filter(|行| *行 == "--user-data-dir").count(),
            3
        );
        assert!(参数.lines().any(|行| 行 == 包.to_str().unwrap()));
        assert!(参数.lines().any(|行| 行 == "--new-window"));
        assert_eq!(
            参数
                .lines()
                .filter(|行| *行 == "--disable-workspace-trust")
                .count(),
            1
        );
        目标.用户环境 = true;
        std::fs::write(会话.join("参数"), "").unwrap();
        安装并打开(&程序, 会话, &包, &目标).unwrap();
        let 参数 = std::fs::read_to_string(会话.join("参数")).unwrap();
        assert!(!参数.contains("--user-data-dir"));
        assert!(!参数.contains("--extensions-dir"));
        assert!(!参数.contains("--disable-workspace-trust"));
        assert!(参数.lines().any(|行| 行 == "--new-window"));
    }
}
