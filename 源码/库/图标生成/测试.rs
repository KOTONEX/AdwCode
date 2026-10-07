// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

use super::几何::{子路径, 字体变换, 展平三次, 线段, 绕数索引, 轮廓转二次};
use super::字体封装::{修补os2版本, 压缩度量数};
use super::矢量解析::{绘制svg, 解析xml, 解析路径数据};
use super::{SVG命名空间, 导入入口, 最大嵌套深度, 校验相对路径, 自有入口};
use kurbo::Point;
use serde_json::Value;

use skrifa::FontRef;
use skrifa::MetadataProvider;
use skrifa::raw::TableProvider;
use std::path::Path as 路径;

/// 建立含图标资产的临时仓库根目录。
fn 测试根() -> tempfile::TempDir {
    let 临时 = tempfile::tempdir().expect("创建临时目录");
    let 产品目录 = 临时.path().join("产品图标");
    std::fs::create_dir_all(&产品目录).expect("创建产品图标目录");
    let 源图标 = 路径::new(env!("CARGO_MANIFEST_DIR")).join("产品图标");
    std::fs::copy(源图标.join("来源.json"), 产品目录.join("来源.json")).expect("复制来源.json");
    复制目录(&源图标.join("符号图标"), &产品目录.join("符号图标"));
    复制目录(&源图标.join("导入图标"), &产品目录.join("导入图标"));
    临时
}

fn 复制目录(来源: &路径, 目标: &路径) {
    std::fs::create_dir_all(目标).expect("创建目录");
    for 项 in std::fs::read_dir(来源).expect("读取目录") {
        let 项 = 项.expect("目录项");
        let 目标路径 = 目标.join(项.file_name());
        if 项.path().is_dir() {
            复制目录(&项.path(), &目标路径);
        } else {
            std::fs::copy(项.path(), 目标路径).expect("复制文件");
        }
    }
}

fn 读取字体(路径: &路径) -> Vec<u8> {
    std::fs::read(路径).expect("读取字体")
}

#[test]
fn 同起终点的闭环曲线不能被展平丢失() {
    let mut 点 = vec![Point::ZERO];
    展平三次(
        Point::ZERO,
        Point::new(10.0, 0.0),
        Point::new(0.0, 10.0),
        Point::ZERO,
        0.1,
        0,
        &mut 点,
    );
    assert!(点.len() > 4);
    assert!(点.iter().any(|点| 点.x > 1.0 && 点.y > 1.0));
}

#[test]
fn 拒绝多个根未闭合xml和越界起点() {
    for 文本 in ["<svg/><svg/>", "<svg/ ><svg>"] {
        assert!(解析xml(文本, "测试").is_err());
    }
    assert!(解析路径数据("M1e100 0", "测试").is_err());
}

#[test]
fn 损坏字体表返回错误而不越界访问() {
    let mut 字节 = vec![0; 44];
    字节[4..6].copy_from_slice(&2u16.to_be_bytes());
    字节[12..16].copy_from_slice(b"OS/2");
    字节[20..24].copy_from_slice(&100u32.to_be_bytes());
    字节[24..28].copy_from_slice(&2u32.to_be_bytes());
    字节[28..32].copy_from_slice(b"head");
    assert!(修补os2版本(&mut 字节, 3).is_err());
}

#[test]
fn 自有字形数量与映射正确() {
    let 根 = 测试根();
    自有入口(根.path(), &[]).expect("生成自有字形");
    let 数据 = 读取字体(&根.path().join("产品图标/adwcode-符号.ttf"));
    let 字体 = FontRef::new(&数据).expect("解析自有字体");
    assert_eq!(字体.maxp().expect("maxp").num_glyphs(), 10);
    let 映射: Vec<(u32, skrifa::GlyphId)> = 字体.charmap().mappings().collect();
    assert_eq!(映射.len(), 9);
    assert!(映射.iter().any(|(码点, _)| *码点 == 0xF001));
    assert!(映射.iter().any(|(码点, _)| *码点 == 0xF009));
}

#[test]
fn 轮廓转二次不会过度细分() {
    let 轮廓 = 子路径 {
        起点: Point::new(4.96875, 1.003906),
        段: vec![线段::三次(
            Point::new(3.324219, 1.003906),
            Point::new(1.96875, 2.359375),
            Point::new(1.96875, 4.003906),
        )],
        闭合: false,
    };
    let 二次 = 轮廓转二次(&轮廓, 1.0);
    assert!(
        二次.段.len() <= 4,
        "普通三次曲线不应过度细分：{}",
        二次.段.len()
    );
}

#[test]
fn 导入字形分组与映射正确() {
    let 根 = 测试根();
    导入入口(根.path(), &[]).expect("生成导入字形");
    for (文件, 字形数, 映射数) in [
        ("adwaita-symbols.ttf", 38, 37),
        ("builder-symbols.ttf", 26, 25),
        ("morewaita-symbols.ttf", 4, 3),
    ] {
        let 数据 = 读取字体(&根.path().join("产品图标").join(文件));
        let 字体 = FontRef::new(&数据).expect("解析导入字体");
        assert_eq!(
            字体.maxp().expect("maxp").num_glyphs(),
            字形数,
            "{文件} 字形数"
        );
        let 映射 = 字体.charmap().mappings().count();
        assert_eq!(映射, 映射数, "{文件} 映射数");
        let os2 = 字体.os2().expect("OS/2");
        assert_eq!(os2.version(), 3, "{文件} 的 OS/2 版本");
    }
}

#[test]
fn 生成结果确定() {
    let 根一 = 测试根();
    let 根二 = 测试根();
    自有入口(根一.path(), &[]).expect("生成自有字形一");
    自有入口(根二.path(), &[]).expect("生成自有字形二");
    assert_eq!(
        读取字体(&根一.path().join("产品图标/adwcode-符号.ttf")),
        读取字体(&根二.path().join("产品图标/adwcode-符号.ttf")),
        "自有字形字体字节不一致"
    );
    导入入口(根一.path(), &[]).expect("生成导入字形一");
    导入入口(根二.path(), &[]).expect("生成导入字形二");
    for 文件 in [
        "adwaita-symbols.ttf",
        "builder-symbols.ttf",
        "morewaita-symbols.ttf",
    ] {
        assert_eq!(
            读取字体(&根一.path().join("产品图标").join(文件)),
            读取字体(&根二.path().join("产品图标").join(文件)),
            "{文件} 字节不一致"
        );
    }
    assert_eq!(
        std::fs::read(
            根一
                .path()
                .join("产品图标/渲染图标/adwcode-adwaita/e300.svg")
        )
        .expect("读取预览一"),
        std::fs::read(
            根二
                .path()
                .join("产品图标/渲染图标/adwcode-adwaita/e300.svg")
        )
        .expect("读取预览二"),
        "预览字节不一致"
    );
}

#[test]
fn 来源路径拒绝越界() {
    for 文本 in ["", "../x.svg", "/etc/passwd", "a/../../b.svg"] {
        assert!(校验相对路径(文本, "glyphs[].file").is_err(), "{文本}");
    }
    assert!(校验相对路径("导入图标/adwaita/x.svg", "glyphs[].file").is_ok());
}

#[test]
fn 闭合后绘制命令另起子路径() {
    let 子路径列表 = 解析路径数据("M0 0L1 0Z L2 2", "测试.svg").expect("解析路径数据");
    assert_eq!(子路径列表.len(), 2);
    assert!(子路径列表[0].闭合);
    assert_eq!(子路径列表[0].段.len(), 1);
    assert_eq!(子路径列表[1].起点, Point::new(0.0, 0.0));
    assert_eq!(子路径列表[1].段.len(), 1);
    assert_eq!(子路径列表[1].段[0].终点(), Point::new(2.0, 2.0));
}

#[test]
fn 路径数据拒绝超幅坐标() {
    assert!(解析路径数据("M0 0L1e100 0", "测试.svg").is_err());
    assert!(解析路径数据("M0 0LNaN 0", "测试.svg").is_err());
}

#[test]
fn xml嵌套过深报错() {
    let 深 = 最大嵌套深度 + 1;
    let 文本 = format!(
        "<svg xmlns=\"{SVG命名空间}\">{}{}</svg>",
        "<g>".repeat(深),
        "</g>".repeat(深)
    );
    assert!(解析xml(&文本, "测试.svg").is_err());
}

#[test]
fn 压缩度量数按末尾等宽收敛() {
    assert_eq!(压缩度量数(&[1024, 1024, 512, 512]), 3);
    assert_eq!(压缩度量数(&[1024, 1024, 1024]), 1);
    assert_eq!(压缩度量数(&[512, 1024]), 2);
}

fn 逐边绕数(多边形列表: &[Vec<Point>], 点: Point) -> i64 {
    let mut 结果 = 0i64;
    for 多边形 in 多边形列表 {
        let 数量 = 多边形.len();
        for 序号 in 0..数量 {
            let 起点 = 多边形[序号];
            let 终点 = 多边形[(序号 + 1) % 数量];
            if 起点.y <= 点.y {
                if 终点.y > 点.y && (终点 - 起点).cross(点 - 起点) > 0.0 {
                    结果 += 1;
                }
            } else if 终点.y <= 点.y && (终点 - 起点).cross(点 - 起点) < 0.0 {
                结果 -= 1;
            }
        }
    }
    结果
}

#[test]
fn 绕数索引与逐边扫描一致() {
    let 多边形列表 = vec![
        vec![
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 4.0),
            Point::new(0.0, 4.0),
        ],
        vec![
            Point::new(1.0, 1.0),
            Point::new(1.0, 3.0),
            Point::new(3.0, 3.0),
            Point::new(3.0, 1.0),
        ],
        vec![
            Point::new(-10.0, -10.0),
            Point::new(-8.0, -10.0),
            Point::new(-9.0, -8.0),
        ],
    ];
    let 索引 = 绕数索引::新建(&多边形列表);
    for 点 in [
        Point::new(0.5, 0.5),
        Point::new(2.0, 2.0),
        Point::new(1.0, 2.0),
        Point::new(5.0, 2.0),
        Point::new(-9.0, -9.5),
        Point::new(2.0, -1.0),
    ] {
        assert_eq!(索引.绕数(点), 逐边绕数(&多边形列表, 点), "{点:?}");
    }
}
#[test]
fn svg继承隐藏透明与奇偶规则() {
    let 绘制 = |内部: &str| {
        let mut 轮廓 = Vec::new();
        绘制svg(
            &format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{内部}</svg>"#),
            "测试.svg",
            &mut 轮廓,
        )
        .map(|_| 轮廓)
    };
    let 方块 = r#"<path d="M2 2H14V14H2Z"/>"#;
    assert!(
        绘制(&format!(r#"<g fill="none">{方块}</g>"#))
            .unwrap()
            .is_empty()
    );
    assert!(
        绘制(&format!(r#"<g style="display:none">{方块}</g>"#))
            .unwrap()
            .is_empty()
    );
    assert!(
        绘制(&format!(r#"<g opacity="0">{方块}</g>"#))
            .unwrap()
            .is_empty()
    );
    assert!(绘制(&format!(r#"<g opacity="0.5">{方块}</g>"#)).is_err());
    assert!(
        !绘制(r#"<g visibility="hidden"><path visibility="visible" d="M2 2H14V14H2Z"/></g>"#)
            .unwrap()
            .is_empty()
    );
    let 轮廓 = 绘制(r#"<path fill-rule="evenodd" d="M2 2H14V14H2Z M4 4H12V12H4Z"/>"#).unwrap();
    let 索引 = 绕数索引::新建(&轮廓.iter().map(|项| 项.展平(0.01)).collect::<Vec<_>>());
    assert_eq!(索引.绕数(字体变换() * Point::new(8.0, 8.0)), 0);
    assert_ne!(索引.绕数(字体变换() * Point::new(3.0, 3.0)), 0);
    let mut 输出 = Vec::new();
    assert!(绘制svg("<g/>", "错误.svg", &mut 输出).is_err());
    assert!(解析路径数据("M0 0 A1e100 1 0 0 0 1 1", "错误.svg").is_err());
    assert!(绘制(r#"<g transform="scale(10000)"><g transform="scale(10000)"><path fill-rule="evenodd" d="M0 0H1V1H0Z"/></g></g>"#).is_err());
    assert!(
        绘制(r#"<g transform="scale(10000)"><path fill-rule="evenodd" d="M0 0H2V2H0Z"/></g>"#)
            .is_err()
    );
}
#[test]
fn 后续字体码点失败不覆盖早先字体() {
    let 临时 = 测试根();
    let 来源路径 = 临时.path().join("产品图标/来源.json");
    let mut 来源: Value = serde_json::from_slice(&std::fs::read(&来源路径).unwrap()).unwrap();
    let 字体 = 来源["fonts"].as_array_mut().unwrap();
    let 首输出 = 临时
        .path()
        .join("产品图标")
        .join(字体[0]["output"].as_str().unwrap());
    std::fs::write(&首输出, "原字体").unwrap();
    字体.last_mut().unwrap()["glyphs"][0]["codepoint"] = Value::String("ffff".to_string());
    std::fs::write(来源路径, serde_json::to_vec(&来源).unwrap()).unwrap();
    assert!(导入入口(临时.path(), &[]).is_err());
    assert_eq!(std::fs::read_to_string(首输出).unwrap(), "原字体");
}
