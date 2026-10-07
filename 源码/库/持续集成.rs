// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 工具已编译后的 CI 辅助步骤；Release 启动阶段仍由工作流校验工具。

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::变更日志::执行git;
use crate::错误::{工具错误, 结果};

fn 需要宿主(文件: &[&str], 前清单: &Value, 后清单: &Value) -> bool {
    if 文件.iter().any(|名| {
        ["扩展/", "类型声明/", "测试/", ".github/workflows/"]
            .iter()
            .any(|前缀| 名.starts_with(前缀))
            || [
                "tsconfig.json",
                "源码/库/宿主测试.rs",
                "源码/库/持续集成.rs",
                "源码/库/运行工具.rs",
                "源码/库/入口.rs",
                "源码/库/命令.rs",
            ]
            .contains(名)
    }) {
        return true;
    }
    if !文件.contains(&"package.json") {
        return false;
    }
    let 去版本 = |值: &Value| {
        值.as_object().map(|对象| {
            let mut 副本 = 对象.clone();
            副本.remove("version");
            副本
        })
    };
    match (去版本(前清单), 去版本(后清单)) {
        (Some(前), Some(后)) => 前 != 后,
        _ => true,
    }
}

fn 改动判断(根目录: &Path, 基线: &str) -> 结果<bool> {
    if !matches!(基线.len(), 40 | 64) || !基线.bytes().all(|字节| 字节.is_ascii_hexdigit())
    {
        return Err(工具错误::新("改动基线不是完整提交摘要"));
    }
    let 文件 = 执行git(根目录, &["diff", "--name-only", "-z", 基线, "HEAD", "--"])?;
    let 文件: Vec<_> = 文件.split('\0').filter(|名| !名.is_empty()).collect();
    // 未改清单时无须读取旧清单；仓库历史导入等情况按实际路径判定。
    if !文件.contains(&"package.json") {
        return Ok(需要宿主(&文件, &Value::Null, &Value::Null));
    }
    let 前清单 = serde_json::from_str(&执行git(
        根目录,
        &["show", &format!("{基线}:package.json")],
    )?)
    .map_err(json错误)?;
    let 后清单 =
        serde_json::from_slice(&std::fs::read(根目录.join("package.json"))?).map_err(json错误)?;
    Ok(需要宿主(&文件, &前清单, &后清单))
}

fn 判断相关(根目录: &Path, 事件名称: &str, 事件: &Value) -> bool {
    if 事件名称 == "workflow_dispatch" {
        return true;
    }
    let 基线 = 事件["pull_request"]["base"]["sha"]
        .as_str()
        .or_else(|| 事件["before"].as_str());
    if let Some(基线) = 基线.filter(|值| !值.is_empty() && !值.bytes().all(|字节| 字节 == b'0'))
    {
        match 改动判断(根目录, 基线) {
            Ok(相关) => return 相关,
            Err(错误) => println!("无法确认改动基线，完整运行宿主测试：{错误}"),
        }
    }
    true
}

fn 最新版本(响应: &str) -> 结果<String> {
    let 版本: Vec<String> = serde_json::from_str(响应).map_err(json错误)?;
    版本
        .into_iter()
        .next()
        .filter(|文本| {
            semver::Version::parse(文本)
                .is_ok_and(|版本| 版本.pre.is_empty() && 版本.build.is_empty())
        })
        .ok_or_else(|| 工具错误::新("最新稳定宿主版本格式错误或列表为空"))
}

fn 验收周(日期: OffsetDateTime) -> String {
    let (年份, 周, _) = 日期.to_offset(time::UtcOffset::UTC).to_iso_week_date();
    format!("{年份:04}-{周:02}")
}

fn 运行宿主(相关: &str, 已验收: &str) -> 结果<bool> {
    if !matches!(相关, "true" | "false") || !matches!(已验收, "true" | "false" | "") {
        return Err(工具错误::新("宿主计划输入不是 Actions 布尔输出"));
    }
    Ok(相关 == "true" || 已验收 != "true")
}

fn 登记工具(根目录: &Path, 程序: &Path, 目录: &Path, 平台: &str) -> 结果<()> {
    crate::文件事务::校验普通路径(目录)?;
    // 每次 CI 使用新的目录，已有产物不得在不知情时被覆盖。
    std::fs::create_dir(目录)?;
    let 数据 = std::fs::read(程序)?;
    let 提交 = 执行git(根目录, &["rev-parse", "HEAD"])?;
    let 元数据 = json!({
        "提交": 提交.trim(), "平台": 平台,
        "sha256": crate::摘要::sha256十六进制(&数据),
    });
    crate::文件事务::写入批次(&[
        (目录.join("adwcode"), Some(数据)),
        (
            目录.join("元数据.json"),
            Some(serde_json::to_vec(&元数据).map_err(json错误)?),
        ),
    ])
}

fn json错误(错误: serde_json::Error) -> 工具错误 {
    工具错误::新(format!("CI JSON 解析或序列化失败：{错误}"))
}

fn 环境(键: &str) -> 结果<String> {
    std::env::var(键).map_err(|错误| 工具错误::新(format!("缺少 CI 环境 {键}：{错误}")))
}

fn 临时路径(文件名: &str) -> 结果<std::path::PathBuf> {
    Ok(Path::new(&环境("RUNNER_TEMP")?).join(文件名))
}

fn 追加输出(文本: &str) -> 结果<()> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(环境("GITHUB_OUTPUT")?)?
        .write_all(文本.as_bytes())?;
    Ok(())
}

/// `adwcode 持续集成 <宿主改动/宿主版本/宿主计划/宿主成功/登记工具>`。
pub fn 入口(根目录: &Path, 参数: &[String]) -> 结果<()> {
    let [步骤] = 参数 else {
        return Err(工具错误::新("持续集成需要且只接受一个步骤名称"));
    };
    match 步骤.as_str() {
        "宿主改动" => {
            let 事件 = serde_json::from_slice(&std::fs::read(环境("GITHUB_EVENT_PATH")?)?)
                .map_err(json错误)?;
            let 相关 = 判断相关(根目录, &环境("GITHUB_EVENT_NAME")?, &事件);
            追加输出(&format!("相关={相关}\n"))
        }
        "宿主版本" => {
            let 代理 = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(30)))
                .build()
                .new_agent();
            let 响应 = 代理
                .get("https://update.code.visualstudio.com/api/releases/stable?released=true")
                .call()
                .map_err(|错误| 工具错误::新(format!("查询宿主稳定版本失败：{错误}")))?
                .body_mut()
                .read_to_string()
                .map_err(|错误| 工具错误::新(format!("读取宿主稳定版本失败：{错误}")))?;
            追加输出(&format!(
                "版本={}\n周={}\n",
                最新版本(&响应)?,
                验收周(OffsetDateTime::now_utc())
            ))
        }
        "宿主计划" => 追加输出(&format!(
            "运行={}\n",
            运行宿主(&环境("相关")?, &环境("已验收")?)?
        )),
        "宿主成功" => {
            let 路径 = 临时路径("adwcode-宿主周记录")?;
            crate::文件事务::校验普通路径(&路径)?;
            std::fs::write(路径, "成功")?;
            Ok(())
        }
        "登记工具" => {
            if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
                return Err(工具错误::新("CI 发布工具目前只登记 linux-x64"));
            }
            登记工具(
                根目录,
                &std::env::current_exe()?,
                &临时路径("adwcode-工具")?,
                "linux-x64",
            )
        }
        _ => Err(工具错误::新(format!("未知持续集成步骤：{步骤}"))),
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::process::Command;

    #[test]
    fn 相关路径与清单语义() {
        assert!(!需要宿主(
            &["README.md", "源码/库/摘要.rs"],
            &json!({}),
            &json!({})
        ));
        assert!(!需要宿主(
            &["package.json"],
            &json!({"version":"1.0.0","main":"入口.js"}),
            &json!({"main":"入口.js","version":"1.1.0"})
        ));
        assert!(需要宿主(
            &["package.json"],
            &json!({"main":"旧.js"}),
            &json!({"main":"新.js"})
        ));
        assert!(需要宿主(&["package.json"], &Value::Null, &Value::Null));
        for 文件 in [
            "扩展/字体.js",
            "测试/真实宿主.cjs",
            "类型声明/宿主.d.ts",
            "tsconfig.json",
            "源码/库/宿主测试.rs",
            "源码/库/持续集成.rs",
            "源码/库/运行工具.rs",
            "源码/库/命令.rs",
            "源码/库/入口.rs",
            ".github/workflows/ci.yml",
        ] {
            assert!(需要宿主(&[文件], &Value::Null, &Value::Null), "{文件}");
        }
    }

    #[test]
    fn 版本响应与跨年验收周() {
        assert_eq!(最新版本(r#"["1.140.0","1.139.0"]"#).unwrap(), "1.140.0");
        for 文本 in [
            "[]",
            "{}",
            r#"["1.2.3\n运行=false"]"#,
            r#"["1.2"]"#,
            r#"["1.2.3-insider"]"#,
            r#"["01.2.3"]"#,
            r#"["1.2.3+build.1"]"#,
        ] {
            assert!(最新版本(文本).is_err());
        }
        for (时间戳, 周) in [
            (1577664000, "2020-01"),
            (1609459200, "2020-53"),
            (1609718400, "2021-01"),
        ] {
            assert_eq!(
                验收周(OffsetDateTime::from_unix_timestamp(时间戳).unwrap()),
                周
            );
        }
    }

    #[test]
    fn 验收计划覆盖缓存与输入错误() {
        for 相关 in ["true", "false"] {
            for 已验收 in ["true", "false", ""] {
                assert_eq!(
                    运行宿主(相关, 已验收).unwrap(),
                    相关 == "true" || 已验收 != "true"
                );
            }
        }
        assert!(运行宿主("", "true").is_err());
        assert!(运行宿主("false", "未知").is_err());
    }

    #[test]
    fn 真实历史与产物登记() {
        let 临时 = tempfile::tempdir().unwrap();
        let 根 = 临时.path();
        let git = |参数: &[&str]| {
            let 输出 = Command::new("git")
                .current_dir(根)
                .args(参数)
                .output()
                .unwrap();
            assert!(
                输出.status.success(),
                "{}",
                String::from_utf8_lossy(&输出.stderr)
            );
            String::from_utf8(输出.stdout).unwrap().trim().to_string()
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "测试"]);
        git(&["config", "user.email", "test@example.invalid"]);
        std::fs::write(
            根.join("package.json"),
            r#"{"version":"1.0.0","main":"入口.js"}"#,
        )
        .unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "初始化"]);
        let 基线 = git(&["rev-parse", "HEAD"]);
        std::fs::write(
            根.join("package.json"),
            r#"{"version":"1.1.0","main":"入口.js"}"#,
        )
        .unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "更新版本"]);
        assert!(!判断相关(根, "push", &json!({"before":基线})));
        assert!(!判断相关(
            根,
            "pull_request",
            &json!({"pull_request":{"base":{"sha":基线}}})
        ));
        assert!(判断相关(
            根,
            "workflow_dispatch",
            &json!({"before":基线})
        ));
        for 事件 in [
            json!({}),
            json!({"before":"0".repeat(40)}),
            json!({"before":"a".repeat(40)}),
            json!({"before":"--错误"}),
        ] {
            assert!(判断相关(根, "push", &事件));
        }
        std::fs::write(
            根.join("package.json"),
            r#"{"version":"1.1.0","main":"新入口.js"}"#,
        )
        .unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "修改入口"]);
        assert!(判断相关(根, "push", &json!({"before":基线})));
        let 程序 = 根.join("工具输入");
        std::fs::write(&程序, "测试工具\0".as_bytes()).unwrap();
        let 目录 = 根.join("产物");
        登记工具(根, &程序, &目录, "linux-x64").unwrap();
        let 数据 = std::fs::read(目录.join("adwcode")).unwrap();
        let 元数据: Value =
            serde_json::from_slice(&std::fs::read(目录.join("元数据.json")).unwrap()).unwrap();
        assert_eq!(元数据["提交"], git(&["rev-parse", "HEAD"]));
        assert_eq!(元数据["平台"], "linux-x64");
        assert_eq!(元数据["sha256"], crate::摘要::sha256十六进制(&数据));
        assert_eq!(数据, std::fs::read(&程序).unwrap());
        assert!(登记工具(根, &程序, &目录, "linux-x64").is_err());
    }
}
