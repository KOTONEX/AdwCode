// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 使用已安装编辑器启动独立 Extension Development Host。
use crate::错误::{工具错误, 结果};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let mut 编辑器: Option<PathBuf> = None;
    let mut 输出 = None;
    let mut 位置 = 0;
    while 位置 < 参数.len() {
        let 槽 = match 参数[位置].as_str() {
            "--编辑器程序" => &mut 编辑器,
            "--输出" => &mut 输出,
            值 => return Err(工具错误::新(format!("宿主测试不接受参数：{值}"))),
        };
        if 槽.is_some() {
            return Err(工具错误::新("宿主测试参数重复"));
        }
        位置 += 1;
        let 值 = 参数
            .get(位置)
            .filter(|值| !值.starts_with("--") && !值.is_empty())
            .ok_or_else(|| 工具错误::新("宿主测试参数缺值"))?;
        *槽 = Some(PathBuf::from(值));
        位置 += 1;
    }
    let 编辑器 = 编辑器
        .or_else(|| crate::工作台基准::查找程序("code"))
        .ok_or_else(|| 工具错误::新("找不到 VS Code"))?;
    let 编辑器 = std::fs::canonicalize(编辑器)?;
    let 编辑器 = if 编辑器
        .parent()
        .is_some_and(|目录| 目录.file_name().is_some_and(|名| 名 == "bin"))
    {
        编辑器.parent().and_then(Path::parent).unwrap().join("code")
    } else {
        编辑器
    };
    let 临时 = tempfile::Builder::new()
        .prefix("adwcode-宿主测试-")
        .tempdir()?;
    let 目录 = 临时.path();
    let 工作区 = 目录.join("工作区");
    std::fs::create_dir(&工作区)?;
    std::fs::write(目录.join("隔离标记"), b"AdwCode")?;
    let 配置 = 目录.join("用户数据");
    std::fs::create_dir_all(配置.join("User"))?;
    std::fs::write(
        配置.join("User/settings.json"),
        r#"{"security.workspace.trust.enabled":false,"workbench.startupEditor":"none","window.confirmBeforeClose":"never","telemetry.telemetryLevel":"off"}"#,
    )?;
    let 日志 = std::fs::File::create(目录.join("启动.log"))?;
    let mut 命令 = Command::new(编辑器);
    命令
        .arg("--user-data-dir")
        .arg(&配置)
        .arg("--extensions-dir")
        .arg(目录.join("扩展"))
        .arg("--new-window")
        .arg("--disable-extensions")
        .arg("--skip-add-to-recently-opened")
        .arg("--disable-workspace-trust")
        .arg(format!("--extensionDevelopmentPath={}", 根目录.display()))
        .arg(format!(
            "--extensionTestsPath={}",
            根目录.join("测试/真实宿主.cjs").display()
        ))
        .arg(&工作区)
        .env("ADWCODE_测试目录", 目录)
        .env("XDG_CONFIG_HOME", 目录.join("配置"))
        .env("XDG_CACHE_HOME", 目录.join("缓存"))
        .env("XDG_DATA_HOME", 目录.join("数据"))
        .stdout(Stdio::from(日志.try_clone()?))
        .stderr(Stdio::from(日志))
        .process_group(0);
    for 键 in [
        "ELECTRON_RUN_AS_NODE",
        "VSCODE_IPC_HOOK_CLI",
        "VSCODE_NLS_CONFIG",
        "VSCODE_CWD",
    ] {
        命令.env_remove(键);
    }
    let mut 进程 = crate::工作台基准::受管进程(命令.spawn()?);
    let 截止 = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(状态) = 进程.0.try_wait()? {
            if !状态.success() {
                return Err(工具错误::新(format!(
                    "宿主测试失败（{状态}）：{}",
                    std::fs::read_to_string(目录.join("启动.log"))?
                )));
            }
            break;
        }
        if Instant::now() > 截止 {
            return Err(工具错误::新("宿主测试超时（120秒）"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let 报告: serde_json::Value =
        serde_json::from_slice(&std::fs::read(目录.join("结果.json")).map_err(|错误| {
            工具错误::新(format!(
                "宿主未生成报告：{错误}；{}",
                std::fs::read_to_string(目录.join("启动.log")).unwrap_or_default()
            ))
        })?)
        .map_err(|错误| 工具错误::带来源("宿主报告不是合法 JSON", 错误.into()))?;
    if 报告["complete"] != true {
        return Err(工具错误::新("宿主测试没有完成"));
    }
    let 输出 = 输出.unwrap_or_else(|| 根目录.join("builddir/宿主测试.json"));
    if let Some(父) = 输出.parent() {
        std::fs::create_dir_all(父)?;
    }
    crate::文件事务::写入批次(&[(
        输出,
        Some(
            serde_json::to_vec_pretty(&报告)
                .map_err(|错误| 工具错误::带来源("无法序列化宿主报告", 错误.into()))?,
        ),
    )])?;
    println!("独立宿主：命令注册、配置作用域与状态命令通过；隔离目录及进程将清理");
    Ok(())
}
