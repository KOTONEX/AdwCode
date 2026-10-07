// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 本地与 CI 共用的 Release/市场编排；外部工具参数及报告协议保持原名。

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::发布策略::{判断发布, 发布判断};
use crate::错误::{工具错误, 结果};

#[derive(Debug, PartialEq, Eq)]
enum 发布目标 {
    附件,
    市场,
}

struct 发布资料 {
    仓库: String,
    发布者: String,
    名称: String,
    标签: String,
    策略: 发布判断,
    包: PathBuf,
    说明: PathBuf,
}

struct 执行输出 {
    成功: bool,
    内容: String,
}

type 执行器<'a> = dyn FnMut(&str, &[String]) -> 结果<执行输出> + 'a;

fn 参数(值: &[&str]) -> Vec<String> {
    值.iter().map(|值| (*值).to_string()).collect()
}

fn 成功执行(执行: &mut 执行器<'_>, 程序: &str, 参数: &[String]) -> 结果<String> {
    let 输出 = 执行(程序, 参数)?;
    if !输出.成功 {
        return Err(工具错误::新(format!("{程序} 发布操作失败")));
    }
    Ok(输出.内容)
}

/// gh --include 的首行提供真实 HTTP 状态；认证、网络、服务端错误不能视为不存在。
fn 查询响应(输出: 执行输出) -> 结果<Option<Value>> {
    let 文本 = 输出.内容.replace("\r\n", "\n");
    let (响应头, 正文) = 文本
        .split_once("\n\n")
        .ok_or_else(|| 工具错误::新("GitHub 响应缺少 HTTP 状态"))?;
    let mut 首行 = 响应头.lines().next().unwrap_or_default().split_whitespace();
    if !首行.next().is_some_and(|协议| 协议.starts_with("HTTP/")) {
        return Err(工具错误::新("GitHub 响应协议无效"));
    }
    match 首行.next() {
        Some("404") => Ok(None),
        Some("200") if 输出.成功 => serde_json::from_str(正文)
            .map(Some)
            .map_err(|_| 工具错误::新("GitHub Release 响应不是有效 JSON")),
        _ => Err(工具错误::新("GitHub 查询失败，未执行发布")),
    }
}

fn 上传附件(资料: &发布资料, 执行: &mut 执行器<'_>) -> 结果<()> {
    let 地址 = format!(
        "repos/{}/releases/tags/{}",
        资料.仓库,
        crate::变更日志::quote(&资料.标签)
    );
    let 现有 = 查询响应(执行("gh", &参数(&["api", &地址, "--include"]))?)?;
    let 包名 = 资料
        .包
        .file_name()
        .and_then(|名字| 名字.to_str())
        .ok_or_else(|| 工具错误::新("VSIX 文件名无效"))?;
    let 包路径 = 资料.包.to_string_lossy();
    let 说明路径 = 资料.说明.to_string_lossy();
    let mut 操作 = match 现有 {
        Some(发布) => {
            if 发布["tag_name"] != 资料.标签 || 发布["draft"] != false {
                return Err(工具错误::新("已有 Release 标签不匹配或仍是草稿"));
            }
            let 附件 = 发布["assets"]
                .as_array()
                .ok_or_else(|| 工具错误::新("Release 附件清单无效"))?;
            if 附件.iter().any(|附件| 附件["name"] == 包名) {
                let 临时 = tempfile::tempdir()?;
                成功执行(
                    执行,
                    "gh",
                    &参数(&[
                        "release",
                        "download",
                        &资料.标签,
                        "--repo",
                        &资料.仓库,
                        "--pattern",
                        包名,
                        "--dir",
                        &临时.path().to_string_lossy(),
                    ]),
                )?;
                if std::fs::read(临时.path().join(包名))? != std::fs::read(&资料.包)? {
                    return Err(工具错误::新(
                        "已有 Release 附件与当前标签打包不一致，停止发布",
                    ));
                }
                println!("复用已有且内容一致的 VSIX 附件");
            } else {
                成功执行(
                    执行,
                    "gh",
                    &参数(&[
                        "release",
                        "upload",
                        &资料.标签,
                        &包路径,
                        "--repo",
                        &资料.仓库,
                    ]),
                )?;
            }
            参数(&["release", "edit", &资料.标签])
        }
        None => 参数(&["release", "create", &资料.标签, &包路径, "--verify-tag"]),
    };
    操作.extend(参数(&[
        "--repo",
        &资料.仓库,
        "--title",
        &资料.标签,
        "--notes-file",
        &说明路径,
    ]));
    操作.push(format!("--prerelease={}", 资料.策略.候选));
    操作.push(format!("--latest={}", !资料.策略.候选));
    成功执行(执行, "gh", &操作)?;
    Ok(())
}

fn 发布市场(
    资料: &发布资料,
    有市场令牌: bool,
    有开放令牌: bool,
    执行: &mut 执行器<'_>,
    查询开放: &mut dyn FnMut(&str) -> 结果<bool>,
) -> 结果<()> {
    if !资料.策略.市场 {
        return Err(工具错误::新(
            "此标签仅允许候选 Release，不允许市场发布",
        ));
    }
    let 包路径 = 资料.包.to_string_lossy();
    if 有市场令牌 {
        // 外部工具原生读取 VSCE_PAT，令牌不进入命令行参数或日志。
        成功执行(
            执行,
            "npx",
            &参数(&[
                "--yes",
                "@vscode/vsce",
                "publish",
                "--packagePath",
                &包路径,
                "--skip-duplicate",
            ]),
        )?;
    } else {
        println!("未配置 VSCE_PAT，跳过 VS Code 扩展市场");
    }
    if 有开放令牌 {
        let 地址 = format!(
            "https://open-vsx.org/api/{}/{}/{}",
            资料.发布者, 资料.名称, 资料.策略.版本
        );
        if 查询开放(&地址)? {
            println!("Open VSX 已存在此版本，跳过");
        } else {
            // ovsx 原生读取 OVSX_PAT；网络错误不会触发重复上传。
            成功执行(执行, "npx", &参数(&["--yes", "ovsx", "publish", &包路径]))?;
        }
    } else {
        println!("未配置 OVSX_PAT，跳过 Open VSX");
    }
    Ok(())
}

fn 查询开放(地址: &str) -> 结果<bool> {
    let 配置 = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build();
    match 配置.new_agent().get(地址).call() {
        Ok(响应) if 响应.status().as_u16() == 200 => Ok(true),
        Err(ureq::Error::StatusCode(404)) => Ok(false),
        _ => Err(工具错误::新("Open VSX 查询失败，未执行上传")),
    }
}

fn 解析参数(输入: &[String]) -> 结果<(String, 发布目标, bool)> {
    let (mut 引用, mut 目标, mut 预演) = (None, None, false);
    let mut 输入 = 输入.iter();
    while let Some(键) = 输入.next() {
        if 键 == "--预演" && !预演 {
            预演 = true;
            continue;
        }
        let 已设置 = match 键.as_str() {
            "--引用" => 引用.is_some(),
            "--目标" => 目标.is_some(),
            _ => return Err(工具错误::新(format!("未知或重复参数：{键}"))),
        };
        if 已设置 {
            return Err(工具错误::新(format!("重复参数：{键}")));
        }
        let 值 = 输入
            .next()
            .filter(|值| !值.is_empty() && !值.starts_with("--"))
            .ok_or_else(|| 工具错误::新(format!("{键} 缺少取值")))?;
        if 键 == "--引用" {
            引用 = Some(值.clone());
        } else {
            目标 = Some(match 值.as_str() {
                "Release" => 发布目标::附件,
                "市场" => 发布目标::市场,
                _ => return Err(工具错误::新("--目标 只接受 Release 或 市场")),
            });
        }
    }
    let 引用 = 引用
        .filter(|引用| 引用.starts_with("refs/tags/v"))
        .ok_or_else(|| 工具错误::新("发布上传需要 --引用 refs/tags/v<版本>"))?;
    Ok((引用, 目标.ok_or_else(|| 工具错误::新("缺少 --目标"))?, 预演))
}

fn 平台标识(值: &Value) -> 结果<&str> {
    值.as_str()
        .filter(|值| {
            !值.is_empty()
                && 值
                    .bytes()
                    .all(|字节| 字节.is_ascii_alphanumeric() || b"_-".contains(&字节))
        })
        .ok_or_else(|| 工具错误::新("发布者或扩展名称无效"))
}

/// 严格标签、干净的打包源文件及现有 VSIX 校验后，才运行外部发布工具。
pub fn 入口(根目录: &Path, 输入: &[String]) -> 结果<()> {
    let (引用, 目标, 预演) = 解析参数(输入)?;
    let 策略 = 判断发布(根目录, &引用, "refs/remotes/origin/main")?;
    if 目标 == 发布目标::市场 && !策略.市场 {
        return Err(工具错误::新("此标签不允许市场发布"));
    }
    let 标签 = 引用.trim_start_matches("refs/tags/").to_string();
    let 说明 = crate::变更日志::生成发布说明(根目录, &策略.版本, Some(&标签))?;
    let mut 干净检查 = Command::new("git");
    干净检查
        .current_dir(根目录)
        .args(["diff", "--quiet", "HEAD", "--"])
        .args(crate::打包::包含清单);
    if !干净检查.status()?.success() {
        return Err(工具错误::新("打包源码有未提交修改，停止发布"));
    }
    let 清单: Value = serde_json::from_slice(&std::fs::read(根目录.join("package.json"))?)
        .map_err(|_| 工具错误::新("清单 JSON 无效"))?;
    let 仓库地址 = 清单["repository"]["url"]
        .as_str()
        .ok_or_else(|| 工具错误::新("清单缺少 GitHub 仓库地址"))?;
    let 仓库 = 仓库地址
        .strip_prefix("https://github.com/")
        .ok_or_else(|| 工具错误::新("发布上传只支持 GitHub 仓库"))?
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let 部分: Vec<_> = 仓库.split('/').collect();
    if 部分.len() != 2
        || 部分
            .iter()
            .any(|部分| 平台标识(&Value::String((*部分).into())).is_err())
    {
        return Err(工具错误::新("GitHub 仓库地址无效"));
    }
    let 发布者 = 平台标识(&清单["publisher"])?;
    let 名称 = 平台标识(&清单["name"])?;
    let 包 = 根目录.join(format!("{名称}-{}.vsix", 策略.版本));
    crate::文件事务::校验普通路径(&包)?;
    let 临时 = tempfile::tempdir()?;
    let 校验包 = 临时.path().join("校验.vsix");
    crate::打包::构建(根目录, &校验包, None)?;
    if std::fs::read(&校验包)? != std::fs::read(&包)? {
        return Err(工具错误::新("VSIX 与当前标签源码不一致，请先重新打包"));
    }
    let 说明路径 = 临时.path().join("发布说明.md");
    std::fs::write(&说明路径, 说明)?;
    let 资料 = 发布资料 {
        仓库: 仓库.into(),
        发布者: 发布者.into(),
        名称: 名称.into(),
        标签,
        策略,
        包,
        说明: 说明路径,
    };
    if 预演 {
        println!(
            "预演通过：{} {}；候选={}；目标={目标:?}；未访问发布服务",
            资料.仓库, 资料.标签, 资料.策略.候选
        );
        return Ok(());
    }
    let mut 执行 = |程序: &str, 参数: &[String]| {
        let 输出 = Command::new(程序).args(参数).current_dir(根目录).output()?;
        Ok(执行输出 {
            成功: 输出.status.success(),
            内容: String::from_utf8_lossy(&输出.stdout).into_owned(),
        })
    };
    match 目标 {
        发布目标::附件 => 上传附件(&资料, &mut 执行),
        发布目标::市场 => 发布市场(
            &资料,
            std::env::var("VSCE_PAT").is_ok_and(|令牌| !令牌.is_empty()),
            std::env::var("OVSX_PAT").is_ok_and(|令牌| !令牌.is_empty()),
            &mut 执行,
            &mut 查询开放,
        ),
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use serde_json::json;

    fn 测试资料(目录: &Path, 市场: bool) -> 发布资料 {
        let 包 = 目录.join("AdwCode-4.2.0.vsix");
        std::fs::write(&包, "固定附件".as_bytes()).unwrap();
        发布资料 {
            仓库: "KOTONEX/AdwCode".into(),
            发布者: "KOTONEX".into(),
            名称: "AdwCode".into(),
            标签: "v4.2.0".into(),
            策略: 发布判断 {
                版本: "4.2.0".into(),
                候选: !市场,
                市场,
            },
            包,
            说明: 目录.join("说明.md"),
        }
    }

    fn 响应(状态: u16, 内容: Value) -> 执行输出 {
        执行输出 {
            成功: 状态 == 200,
            内容: format!("HTTP/2.0 {状态} 测试\r\nContent-Type: application/json\r\n\r\n{内容}"),
        }
    }

    #[test]
    fn 查询只将明确404视为不存在() {
        assert!(查询响应(响应(404, json!({}))).unwrap().is_none());
        assert!(查询响应(响应(200, json!({}))).unwrap().is_some());
        for 状态 in [401, 403, 429, 500] {
            assert!(查询响应(响应(状态, json!({}))).is_err());
        }
        assert!(
            查询响应(执行输出 {
                成功: false,
                内容: "网络中断".into()
            })
            .is_err()
        );
        assert!(
            查询响应(执行输出 {
                成功: true,
                内容: "HTTP/2.0 200\n\n{".into()
            })
            .is_err()
        );
    }

    #[test]
    fn 新建与补充附件使用候选权限且失败不继续() {
        let 临时 = tempfile::tempdir().unwrap();
        let 资料 = 测试资料(临时.path(), false);
        for 已存在 in [false, true] {
            let mut 调用 = vec![];
            上传附件(&资料, &mut |程序, 参数| {
                assert_eq!(程序, "gh");
                调用.push(参数.to_vec());
                Ok(if 参数[0] == "api" {
                    if 已存在 {
                        响应(
                            200,
                            json!({"tag_name": "v4.2.0", "draft": false, "assets": []}),
                        )
                    } else {
                        响应(404, json!({}))
                    }
                } else {
                    执行输出 {
                        成功: true,
                        内容: String::new(),
                    }
                })
            })
            .unwrap();
            let 末次 = 调用.last().unwrap();
            assert!(末次.contains(&"--prerelease=true".into()));
            assert!(末次.contains(&"--latest=false".into()));
            assert_eq!(末次[1], if 已存在 { "edit" } else { "create" });
            if 已存在 {
                assert_eq!(调用[1][1], "upload");
            }
        }
        let mut 次数 = 0;
        assert!(
            上传附件(&资料, &mut |_, _| {
                次数 += 1;
                Ok(响应(403, json!({})))
            })
            .is_err()
        );
        assert_eq!(次数, 1);
        let mut 次数 = 0;
        assert!(
            上传附件(&资料, &mut |_, _| {
                次数 += 1;
                Ok(if 次数 == 1 {
                    响应(
                        200,
                        json!({"tag_name": "v4.2.0", "draft": false, "assets": []}),
                    )
                } else {
                    执行输出 {
                        成功: false,
                        内容: String::new(),
                    }
                })
            })
            .is_err()
        );
        assert_eq!(次数, 2);
    }

    #[test]
    fn 已有附件必须字节一致才更新说明() {
        let 临时 = tempfile::tempdir().unwrap();
        let 资料 = 测试资料(临时.path(), false);
        for 一致 in [true, false] {
            let mut 调用 = vec![];
            let 结果 = 上传附件(&资料, &mut |_, 参数| {
                调用.push(参数.to_vec());
                if 参数[0] == "api" {
                    return Ok(响应(
                        200,
                        json!({"tag_name": "v4.2.0", "draft": false,
                        "assets": [{"name": "AdwCode-4.2.0.vsix"}]}),
                    ));
                }
                if 参数[1] == "download" {
                    let 目录 = &参数[参数.iter().position(|值| 值 == "--dir").unwrap() + 1];
                    std::fs::write(
                        Path::new(目录).join("AdwCode-4.2.0.vsix"),
                        if 一致 {
                            "固定附件".as_bytes()
                        } else {
                            "不同附件".as_bytes()
                        },
                    )
                    .unwrap();
                }
                Ok(执行输出 {
                    成功: true,
                    内容: String::new(),
                })
            });
            assert_eq!(结果.is_ok(), 一致);
            assert_eq!(调用.len(), if 一致 { 3 } else { 2 });
            assert!(
                !调用
                    .iter()
                    .any(|调用| 调用.get(1).is_some_and(|值| 值 == "upload"))
            );
        }
    }

    #[test]
    fn 市场令牌缺失重复与网络失败() {
        let 临时 = tempfile::tempdir().unwrap();
        let 资料 = 测试资料(临时.path(), true);
        let mut 调用 = vec![];
        let mut 执行 = |程序: &str, 参数: &[String]| {
            assert_eq!(程序, "npx");
            assert!(!参数.iter().any(|值| 值 == "-p" || 值 == "--pat"));
            调用.push(参数.to_vec());
            Ok(执行输出 {
                成功: true,
                内容: String::new(),
            })
        };
        发布市场(&资料, false, false, &mut 执行, &mut |_| {
            panic!("无令牌不得查询")
        })
        .unwrap();
        发布市场(&资料, true, true, &mut 执行, &mut |_| Ok(true)).unwrap();
        发布市场(&资料, false, true, &mut 执行, &mut |_| Ok(false)).unwrap();
        assert!(
            发布市场(&资料, false, true, &mut 执行, &mut |_| Err(
                工具错误::新("离线")
            ))
            .is_err()
        );
        assert_eq!(调用.len(), 2);
        assert!(调用[0].contains(&"--skip-duplicate".into()));
        assert!(调用[1].contains(&"ovsx".into()));
        assert!(
            发布市场(
                &测试资料(临时.path(), false),
                true,
                true,
                &mut |_, _| panic!("候选不得发布"),
                &mut |_| panic!("候选不得查询")
            )
            .is_err()
        );
    }

    #[test]
    fn 真实标签预演校验包与未提交源码() {
        let 源码 = crate::仓库::根目录().unwrap();
        let 临时 = tempfile::tempdir().unwrap();
        let 根 = 临时.path().join("源码");
        assert!(
            Command::new("git")
                .args(["clone", "--quiet", "--no-hardlinks"])
                .arg(&源码)
                .arg(&根)
                .status()
                .unwrap()
                .success()
        );
        let git = |参数: &[&str]| crate::变更日志::执行git(&根, 参数).unwrap();
        // CI 通常以分离 HEAD 检出；本地 clone 不会自动复制来源仓库的远端引用。
        let 主线 = crate::变更日志::执行git(
            &源码,
            &["rev-parse", "--verify", "refs/remotes/origin/main"],
        )
        .unwrap();
        git(&["update-ref", "refs/remotes/origin/main", 主线.trim()]);
        git(&["config", "user.name", "离线测试"]);
        git(&["config", "user.email", "test@example.invalid"]);
        git(&["config", "commit.gpgsign", "false"]);
        let mut 清单: Value =
            serde_json::from_slice(&std::fs::read(根.join("package.json")).unwrap()).unwrap();
        清单["version"] = json!("99.0.0");
        std::fs::write(根.join("package.json"), 清单.to_string()).unwrap();
        git(&["add", "package.json"]);
        git(&["commit", "-m", "测试: 发布预演"]);
        git(&["tag", "v99.0.0"]);
        let 输入 = 参数(&["--引用", "refs/tags/v99.0.0", "--目标", "Release", "--预演"]);
        assert!(入口(&根, &输入).is_err()); // 包缺失。
        let 包 = 根.join("AdwCode-99.0.0.vsix");
        crate::打包::构建(&根, &包, None).unwrap();
        入口(&根, &输入).unwrap(); // 无 gh/npx/HTTP 调用。
        std::fs::write(根.join(".vscode/settings.json"), "用户私有配置").unwrap();
        入口(&根, &输入).unwrap(); // 不参与打包的用户配置可保留。
        let 包字节 = std::fs::read(&包).unwrap();
        std::fs::write(&包, "旧版本附件").unwrap();
        assert!(
            入口(&根, &输入)
                .unwrap_err()
                .消息
                .contains("VSIX 与当前标签源码不一致")
        );
        std::fs::write(&包, 包字节).unwrap();
        std::fs::write(根.join("README.md"), "未提交修改").unwrap();
        assert!(入口(&根, &输入).unwrap_err().消息.contains("未提交修改"));
        assert!(
            入口(
                &根,
                &参数(&["--引用", "refs/tags/v99.0.0", "--目标", "市场", "--预演"])
            )
            .is_err()
        );
        assert!(
            入口(
                &根,
                &参数(&["--引用", "refs/tags/v98.0.0", "--目标", "Release", "--预演"])
            )
            .is_err()
        );
    }

    #[test]
    fn 上传参数严格且预演可复用() {
        for 输入 in [
            vec![],
            vec!["--引用", "refs/heads/rust", "--目标", "Release"],
            vec!["--引用", "refs/tags/v4.2.0"],
            vec!["--目标", "别处"],
            vec!["--预演", "--预演"],
            vec!["--引用", "--目标"],
            vec!["--引用", "refs/tags/v4.2.0", "--引用", "refs/tags/v4.2.0"],
            vec!["--目标", "Release", "--目标", "市场"],
            vec!["多余"],
        ] {
            assert!(解析参数(&参数(&输入)).is_err());
        }
        let (_, 目标, 预演) = 解析参数(&参数(&[
            "--引用",
            "refs/tags/v4.2.0",
            "--目标",
            "Release",
            "--预演",
        ]))
        .unwrap();
        assert_eq!(目标, 发布目标::附件);
        assert!(预演);
    }
}
