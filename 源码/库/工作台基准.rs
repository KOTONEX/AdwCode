// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 启动独立 VS Code 窗口，比较外观 CSS 开关；不安装加载器、不执行重载。
//!
//! 测量逻辑仍在 `基准/工作台基准.cjs`（Node + Playwright），本模块只负责
//! 隔离环境、启动调试端口、执行测量并清理本次创建的进程组。

use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use crate::错误::{工具错误, 结果};

/// 在 PATH 中查找可执行文件。
#[must_use]
pub fn 查找程序(名称: &str) -> Option<PathBuf> {
    let 路径 = std::env::var_os("PATH")?;
    std::env::split_paths(&路径)
        .map(|目录| 目录.join(名称))
        .find(|候选| 候选.is_file())
}

fn 参数错误(消息: &str) -> 工具错误 {
    工具错误::新(消息)
}

/// 临时隔离环境；离开作用域时清理。
struct 隔离环境 {
    目录: PathBuf,
}

impl Drop for 隔离环境 {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.目录);
    }
}

fn 复制目录(来源: &Path, 目标: &Path) -> 结果<()> {
    std::fs::create_dir_all(目标)?;
    for 条目 in std::fs::read_dir(来源)? {
        let 条目 = 条目?;
        let 来源路径 = 条目.path();
        let 目标路径 = 目标.join(条目.file_name());
        if 来源路径.is_dir() {
            复制目录(&来源路径, &目标路径)?;
        } else if 来源路径.is_file() {
            std::fs::copy(&来源路径, &目标路径)?;
        }
    }
    Ok(())
}

/// `adwcode 工作台基准` 的入口。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let mut 浏览器工具: Option<PathBuf> = None;
    let mut 编辑器程序: Option<PathBuf> = None;
    let mut 次数 = 6usize;
    let mut 输出路径 = 根目录.join("builddir/performance-ui.json");
    let mut 仅选择器 = false;
    let mut 参考样式: Option<PathBuf> = None;
    let mut 场景 = "all".to_string();
    let mut 序号 = 0;
    while 序号 < 参数.len() {
        let 当前 = 参数[序号].as_str();
        match 当前 {
            "--浏览器工具" | "--编辑器程序" | "--次数" | "--输出" | "--参考样式" | "--场景" =>
            {
                序号 += 1;
                let 值 = 参数
                    .get(序号)
                    .ok_or_else(|| 参数错误(&format!("{当前} 缺少取值")))?;
                match 当前 {
                    "--浏览器工具" => 浏览器工具 = Some(PathBuf::from(值)),
                    "--编辑器程序" => 编辑器程序 = Some(PathBuf::from(值)),
                    "--次数" => {
                        次数 = 值.parse().map_err(|_| 参数错误("--次数 需要整数"))?;
                    }
                    "--输出" => 输出路径 = PathBuf::from(值),
                    "--参考样式" => 参考样式 = Some(PathBuf::from(值)),
                    "--场景" => 场景 = 值.clone(),
                    _ => unreachable!(),
                }
            }
            "--仅选择器" => 仅选择器 = true,
            其他 => return Err(参数错误(&format!("未知参数：{其他}"))),
        }
        序号 += 1;
    }
    if 次数 < 4 {
        return Err(参数错误("--次数 至少为 4"));
    }
    if 仅选择器 && 场景 != "all" {
        return Err(参数错误("--仅选择器 不能与单独滚动场景同时使用"));
    }
    if !matches!(场景.as_str(), "all" | "scroll") {
        return Err(参数错误("--场景 只能是 all 或 scroll"));
    }
    let 浏览器工具 = 浏览器工具.ok_or_else(|| 参数错误("--浏览器工具 为必填"))?;
    let 编辑器命令 = 编辑器程序
        .or_else(|| 查找程序("code"))
        .ok_or_else(|| 参数错误("未找到 code 命令，可用 --编辑器程序 指定"))?;
    let 编辑器 = std::fs::canonicalize(&编辑器命令)
        .map_err(|错误| 工具错误::带来源("无法解析编辑器路径".to_string(), 错误))?
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| 参数错误("无法推导编辑器安装目录"))?
        .join("code");
    let node = 查找程序("node").ok_or_else(|| 参数错误("需要 Node.js"))?;
    if !编辑器.is_file() || !浏览器工具.is_dir() {
        return Err(参数错误(
            "需要本机 Linux VS Code、Node.js 与独立安装的 playwright",
        ));
    }
    输出路径 = std::fs::canonicalize(&输出路径).unwrap_or(输出路径);
    if let Some(父目录) = 输出路径.parent() {
        std::fs::create_dir_all(父目录)?;
    }
    let 隔离 = 隔离环境 {
        目录: std::env::temp_dir().join(format!("adwcode-ui-performance-{}", std::process::id())),
    };
    let 文件夹 = &隔离.目录;
    let _ = std::fs::remove_dir_all(文件夹);
    let 配置 = 文件夹.join("profile");
    let 扩展目录 = 文件夹.join("extensions");
    let 工作区 = 文件夹.join("workspace");
    std::fs::create_dir_all(配置.join("User"))?;
    std::fs::create_dir_all(&工作区)?;

    let 清单文本 = std::fs::read_to_string(根目录.join("package.json"))?;
    let manifest: Value = serde_json::from_str(&清单文本)
        .map_err(|错误| 工具错误::新(format!("package.json 解析失败：{错误}")))?;
    let 扩展路径 = 扩展目录.join(format!(
        "{}.{}-{}",
        manifest["publisher"].as_str().unwrap_or(""),
        manifest["name"].as_str().unwrap_or(""),
        manifest["version"].as_str().unwrap_or("")
    ));
    std::fs::create_dir_all(&扩展路径)?;
    for 名称 in ["扩展", "主题", "附加外观", "产品图标", "资产"] {
        let 来源 = 根目录.join(名称);
        if 来源.exists() {
            复制目录(&来源, &扩展路径.join(名称))?;
        }
    }
    std::fs::copy(根目录.join("package.json"), 扩展路径.join("package.json"))?;

    let mut 设置 = Map::new();
    for (键, 值) in [
        ("workbench.colorTheme", json!("AdwCode 浅色")),
        ("workbench.productIconTheme", json!("adwcode")),
        ("window.autoDetectColorScheme", json!(false)),
        ("window.autoDetectHighContrast", json!(false)),
        ("workbench.startupEditor", json!("none")),
        ("window.restoreWindows", json!("none")),
        ("security.workspace.trust.enabled", json!(false)),
        ("telemetry.telemetryLevel", json!("off")),
        ("update.mode", json!("none")),
        ("extensions.autoCheckUpdates", json!(false)),
        ("extensions.autoUpdate", json!(false)),
        ("editor.minimap.enabled", json!(false)),
        ("editor.stickyScroll.enabled", json!(false)),
        ("editor.smoothScrolling", json!(false)),
        ("workbench.iconTheme", Value::Null),
        ("window.commandCenter", json!(false)),
    ] {
        设置.insert(键.to_string(), 值);
    }
    std::fs::write(
        配置.join("User/settings.json"),
        crate::主题生成::写json(&Value::Object(设置)),
    )?;
    let mut 示例 = String::new();
    for 编号 in 0..2000 {
        示例.push_str(&format!(
            "def sample_{编号}(value: int) -> int:\n    return value + {编号}\n\n"
        ));
    }
    std::fs::write(工作区.join("sample.py"), 示例)?;
    for 编号 in 0..200 {
        std::fs::write(
            工作区.join(format!("file_{编号:03}.txt")),
            format!("文件 {编号}\n"),
        )?;
    }
    let 端口 = {
        let 监听 = TcpListener::bind(("127.0.0.1", 0))?;
        监听.local_addr()?.port()
    };
    let 排除: [&str; 4] = [
        "ELECTRON_RUN_AS_NODE",
        "VSCODE_IPC_HOOK_CLI",
        "VSCODE_NLS_CONFIG",
        "VSCODE_CWD",
    ];
    let 日志路径 = 文件夹.join("launch.log");
    let 日志 = std::fs::File::create(&日志路径)?;
    let mut 启动命令 = Command::new(&编辑器);
    启动命令
        .arg("--user-data-dir")
        .arg(&配置)
        .arg("--extensions-dir")
        .arg(&扩展目录)
        .arg("--new-window")
        .arg("--skip-add-to-recently-opened")
        .arg("--locale=en")
        .arg("--remote-debugging-address=127.0.0.1")
        .arg(format!("--remote-debugging-port={端口}"))
        .arg("--disable-renderer-backgrounding")
        .arg("--disable-background-timer-throttling")
        .arg("--disable-backgrounding-occluded-windows")
        .arg(&工作区)
        .arg(工作区.join("sample.py"))
        .stdout(Stdio::from(日志.try_clone()?))
        .stderr(Stdio::from(日志))
        .process_group(0);
    for (键, _) in std::env::vars_os() {
        if 排除
            .iter()
            .any(|排除键| 键.as_os_str() == std::ffi::OsStr::new(*排除键))
        {
            启动命令.env_remove(键);
        }
    }
    let mut 编辑器进程 = 启动命令
        .spawn()
        .map_err(|错误| 工具错误::带来源("无法启动 VS Code".to_string(), 错误))?;
    let mut session = Map::new();
    session.insert("directory".to_string(), json!(文件夹.to_string_lossy()));
    session.insert("port".to_string(), json!(端口));
    session.insert("pid".to_string(), json!(编辑器进程.id()));
    session.insert("profile".to_string(), json!(配置.to_string_lossy()));
    session.insert("fixture".to_string(), json!(工作区.to_string_lossy()));
    session.insert(
        "referenceCss".to_string(),
        参考样式
            .map(|路径| json!(路径.to_string_lossy()))
            .unwrap_or(Value::Null),
    );
    session.insert("scenario".to_string(), json!(场景));
    let session路径 = 文件夹.join("session.json");
    std::fs::write(
        &session路径,
        crate::主题生成::写json(&Value::Object(session)),
    )?;

    let mut 测量 = Command::new(&node)
        .arg(根目录.join("基准/工作台基准.cjs"))
        .arg(&session路径)
        .arg(std::fs::canonicalize(&浏览器工具)?)
        .arg(&输出路径)
        .arg(次数.to_string())
        .arg(仅选择器.to_string())
        .spawn()
        .map_err(|错误| 工具错误::带来源("无法运行工作台基准".to_string(), 错误))?;
    let 截止 = Instant::now() + Duration::from_secs(240);
    let mut 超时 = false;
    let mut 测量状态 = None;
    while 测量状态.is_none() {
        if let Some(状态) = 测量.try_wait()? {
            测量状态 = Some(状态);
            break;
        }
        if Instant::now() > 截止 {
            let _ = 测量.kill();
            let _ = 测量.wait();
            超时 = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // 只终止本次创建的独立进程组；不连接操作者的窗口。
    if 编辑器进程.try_wait()?.is_none() {
        终止进程组(编辑器进程.id(), "-TERM");
        let 截止 = Instant::now() + Duration::from_secs(10);
        loop {
            if 编辑器进程.try_wait()?.is_some() {
                break;
            }
            if Instant::now() > 截止 {
                终止进程组(编辑器进程.id(), "-KILL");
                let _ = 编辑器进程.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if 超时 {
        return Err(工具错误::新("工作台基准超时（240 秒）"));
    }
    if !测量状态.is_some_and(|状态| 状态.success()) {
        return Err(工具错误::新("工作台基准失败"));
    }
    Ok(())
}

fn 终止进程组(pid: u32, 信号: &str) {
    let _ = Command::new("kill")
        .arg(信号)
        .arg(format!("-{pid}"))
        .status();
}
