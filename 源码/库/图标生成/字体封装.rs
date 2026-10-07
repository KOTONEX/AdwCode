// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者

use super::几何::子路径;
use super::{前进宽度, 固定时间戳};
use crate::错误::{工具错误, 结果};
use kurbo::BezPath;
use write_fonts::FontBuilder;
use write_fonts::tables::cmap::{Cmap, CmapSubtable, EncodingRecord, PlatformId};
use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::head::{Flags, Head, MacStyle};
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::{Hmtx, LongMetric};
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::name::{Name, NameRecord};
use write_fonts::tables::os2::{Os2, SelectionFlags};
use write_fonts::tables::post::Post;
use write_fonts::types::{Fixed, LongDateTime, NameId, Tag};

/// 字体名称表内容。
pub(super) struct 字体描述<'a> {
    pub(super) 族名: &'a str,
    pub(super) 样式名: &'a str,
    pub(super) 唯一标识: &'a str,
    pub(super) 全名: &'a str,
    pub(super) ps名: &'a str,
    pub(super) 版本: &'a str,
    pub(super) 版权: &'a str,
    pub(super) 许可: &'a str,
    pub(super) 许可地址: &'a str,
}

/// 写入 TTF：glyf/loca、度量、cmap、名称与其余必需表。
pub(super) fn 生成字体字节(
    名称表: &[String],
    轮廓表: &[Vec<子路径>],
    码点表: &[(u16, usize)],
    描述: &字体描述<'_>,
) -> 结果<Vec<u8>> {
    if 名称表.len() != 轮廓表.len() || 名称表.first().map(String::as_str) != Some(".notdef")
    {
        return Err(工具错误::新("字形表与名称表不一致"));
    }
    let mut 唯一码点 = std::collections::HashSet::new();
    for &(码点, 字形) in 码点表 {
        if char::from_u32(u32::from(码点)).is_none()
            || 码点 == 0xffff
            || !唯一码点.insert(码点)
            || 字形 == 0
            || 字形 >= 轮廓表.len()
        {
            return Err(工具错误::新("字体码点重复、无效或映射字形越界"));
        }
    }
    let mut 构建器 = GlyfLocaBuilder::new();
    let mut 包围盒列表: Vec<Option<(i16, i16, i16, i16)>> = Vec::with_capacity(轮廓表.len());
    let mut 最大点数 = 0u16;
    let mut 最大轮廓数 = 0u16;
    for 轮廓 in 轮廓表 {
        if 轮廓.is_empty() {
            构建器
                .add_glyph(&Glyph::Empty)
                .map_err(|错误| 工具错误::新(format!("字形构建失败：{错误}")))?;
            包围盒列表.push(None);
            continue;
        }
        let 路径 = 合并路径(轮廓);
        let 字形 = SimpleGlyph::from_bezpath(&路径)
            .map_err(|错误| 工具错误::新(format!("字形编译失败：{错误:?}")))?;
        if 字形.contours.len() >= i16::MAX as usize {
            return Err(工具错误::新(format!(
                "字形轮廓数过多：{}",
                字形.contours.len()
            )));
        }
        let 点数: usize = 字形.contours.iter().map(|项| 项.len()).sum();
        最大点数 = 最大点数.max(u16::try_from(点数).unwrap_or(u16::MAX));
        最大轮廓数 = 最大轮廓数.max(u16::try_from(字形.contours.len()).unwrap_or(u16::MAX));
        let 盒 = 字形.bbox;
        包围盒列表.push(Some((盒.x_min, 盒.y_min, 盒.x_max, 盒.y_max)));
        构建器
            .add_glyph(&Glyph::Simple(字形))
            .map_err(|错误| 工具错误::新(format!("字形构建失败：{错误}")))?;
    }
    let (glyf, loca, 定位格式) = 构建器.build();

    let 左边界: Vec<i16> = 包围盒列表
        .iter()
        .map(|盒| 盒.map_or(0, |值| 值.0))
        .collect();
    let 前进列表: Vec<u16> = vec![前进宽度; 名称表.len()];
    let 度量数 = 压缩度量数(&前进列表).max(1);
    let 长度量: Vec<LongMetric> = (0..度量数)
        .map(|序号| LongMetric::new(前进宽度, 左边界[序号]))
        .collect();
    let 余留边界: Vec<i16> = 左边界[度量数..].to_vec();
    let hmtx = Hmtx::new(长度量, 余留边界);

    let 非空: Vec<usize> = (0..名称表.len())
        .filter(|序号| 包围盒列表[*序号].is_some())
        .collect();
    let 最小左 = 非空.iter().map(|序号| 左边界[*序号]).min().unwrap_or(0);
    let 最小右 = 非空
        .iter()
        .map(|序号| {
            let 盒 = 包围盒列表[*序号].expect("非空字形有包围盒");
            前进宽度 as i32 - 左边界[*序号] as i32 - (盒.2 as i32 - 盒.0 as i32)
        })
        .min()
        .map_or(0, |值| 值 as i16);
    let 最大延伸 = 非空
        .iter()
        .map(|序号| {
            let 盒 = 包围盒列表[*序号].expect("非空字形有包围盒");
            左边界[*序号] as i32 + (盒.2 as i32 - 盒.0 as i32)
        })
        .max()
        .map_or(0, |值| 值 as i16);
    let hhea = Hhea {
        ascender: 896.into(),
        descender: (-128).into(),
        line_gap: 0.into(),
        advance_width_max: 前进宽度.into(),
        min_left_side_bearing: 最小左.into(),
        min_right_side_bearing: 最小右.into(),
        x_max_extent: 最大延伸.into(),
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        number_of_h_metrics: u16::try_from(度量数).unwrap_or(u16::MAX),
    };

    let maxp = Maxp {
        num_glyphs: u16::try_from(名称表.len()).unwrap_or(u16::MAX),
        max_points: Some(最大点数),
        max_contours: Some(最大轮廓数),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(2),
        max_twilight_points: Some(0),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(0),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };

    let mut 全局盒 = (0i32, 0i32, 0i32, 0i32);
    let mut 有盒 = false;
    for 盒 in 包围盒列表.iter().flatten() {
        if !有盒 {
            全局盒 = (盒.0 as i32, 盒.1 as i32, 盒.2 as i32, 盒.3 as i32);
            有盒 = true;
        } else {
            全局盒.0 = 全局盒.0.min(盒.0 as i32);
            全局盒.1 = 全局盒.1.min(盒.1 as i32);
            全局盒.2 = 全局盒.2.max(盒.2 as i32);
            全局盒.3 = 全局盒.3.max(盒.3 as i32);
        }
    }
    let head = Head {
        font_revision: Fixed::from_f64(1.0),
        flags: Flags::BASELINE_AT_Y_0 | Flags::LSB_AT_X_0,
        units_per_em: 1024,
        created: LongDateTime::new(固定时间戳),
        modified: LongDateTime::new(固定时间戳),
        x_min: 全局盒.0 as i16,
        y_min: 全局盒.1 as i16,
        x_max: 全局盒.2 as i16,
        y_max: 全局盒.3 as i16,
        mac_style: MacStyle::empty(),
        lowest_rec_ppem: 3,
        font_direction_hint: 2,
        index_to_loc_format: 定位格式 as i16,
        ..Default::default()
    };

    let cmap子表 = 构建cmap4(码点表);
    let cmap = Cmap::new(vec![
        EncodingRecord::new(PlatformId::Unicode, 3, cmap子表.clone()),
        EncodingRecord::new(PlatformId::Windows, 1, cmap子表),
    ]);

    let 记录 = vec![
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::COPYRIGHT_NOTICE,
            描述.版权.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::FAMILY_NAME,
            描述.族名.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::SUBFAMILY_NAME,
            描述.样式名.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::UNIQUE_ID,
            描述.唯一标识.to_string().into(),
        ),
        NameRecord::new(3, 1, 0x409, NameId::FULL_NAME, 描述.全名.to_string().into()),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::VERSION_STRING,
            描述.版本.to_string().into(),
        ),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::POSTSCRIPT_NAME,
            描述.ps名.to_string().into(),
        ),
        NameRecord::new(3, 1, 0x409, NameId::new(13), 描述.许可.to_string().into()),
        NameRecord::new(
            3,
            1,
            0x409,
            NameId::new(14),
            描述.许可地址.to_string().into(),
        ),
    ];
    let name = Name::new(记录);

    let 最小码点 = 码点表.iter().map(|项| 项.0).min().unwrap_or(0);
    let 最大码点 = 码点表.iter().map(|项| 项.0).max().unwrap_or(0);
    let os2 = Os2 {
        us_weight_class: 400,
        us_width_class: 5,
        fs_type: 4,
        ach_vend_id: Tag::new(b"????"),
        fs_selection: SelectionFlags::empty(),
        us_first_char_index: 最小码点,
        us_last_char_index: 最大码点,
        s_typo_ascender: 896,
        s_typo_descender: -128,
        s_typo_line_gap: 0,
        us_win_ascent: 896,
        us_win_descent: 128,
        ul_code_page_range_1: Some(0),
        ul_code_page_range_2: Some(0),
        us_default_char: Some(0),
        us_break_char: Some(32),
        us_max_context: Some(0),
        sx_height: Some(0),
        s_cap_height: Some(0),
        x_avg_char_width: 前进宽度 as i16,
        ul_unicode_range_2: 1 << 28,
        ..Default::default()
    };

    let post = Post::new_v2(名称表.iter().map(String::as_str));

    let mut 字体构建 = FontBuilder::new();
    字体构建
        .add_table(&head)
        .and_then(|项| 项.add_table(&hhea))
        .and_then(|项| 项.add_table(&hmtx))
        .and_then(|项| 项.add_table(&maxp))
        .and_then(|项| 项.add_table(&cmap))
        .and_then(|项| 项.add_table(&name))
        .and_then(|项| 项.add_table(&os2))
        .and_then(|项| 项.add_table(&post))
        .and_then(|项| 项.add_table(&glyf))
        .and_then(|项| 项.add_table(&loca))
        .map_err(|错误| 工具错误::新(format!("字体表写入失败：{错误}")))?;
    let mut 字节 = 字体构建.build();
    修补os2版本(&mut 字节, 3)?;
    Ok(字节)
}

/// 把 OS/2 版本改写为指定值并同步校验和（write-fonts 只能按字段算出 4）。
pub(super) fn 修补os2版本(字节: &mut [u8], 版本: u16) -> 结果<()> {
    let 表数 = u16::from_be_bytes(
        字节
            .get(4..6)
            .ok_or_else(|| 工具错误::新("字体缺少表目录"))?
            .try_into()
            .expect("表数"),
    ) as usize;
    let mut os2记录 = None;
    let mut head偏移 = None;
    for 序号 in 0..表数 {
        let 记录 = 12 + 序号 * 16;
        let 标签 = 字节
            .get(记录..记录 + 4)
            .ok_or_else(|| 工具错误::新("字体表目录不完整"))?;
        let 偏移 = u32::from_be_bytes(
            字节
                .get(记录 + 8..记录 + 12)
                .ok_or_else(|| 工具错误::新("字体表目录不完整"))?
                .try_into()
                .expect("表偏移"),
        ) as usize;
        let 长度 = u32::from_be_bytes(
            字节
                .get(记录 + 12..记录 + 16)
                .ok_or_else(|| 工具错误::新("字体表目录不完整"))?
                .try_into()
                .expect("表长度"),
        ) as usize;
        if 标签 == b"OS/2" {
            os2记录 = Some((记录, 偏移, 长度));
        } else if 标签 == b"head" {
            head偏移 = Some(偏移);
        }
    }
    let (记录, 偏移, 长度) = os2记录.ok_or_else(|| 工具错误::新("字体缺少 OS/2 表"))?;
    if 长度 < 2 || 偏移.checked_add(长度).is_none_or(|结束| 结束 > 字节.len()) {
        return Err(工具错误::新("OS/2 表超出字体范围"));
    }
    字节[偏移..偏移 + 2].copy_from_slice(&版本.to_be_bytes());
    let 表校验 = 表校验和(
        字节
            .get(偏移..偏移 + 长度)
            .ok_or_else(|| 工具错误::新("OS/2 表超出字体范围"))?,
    );
    字节[记录 + 4..记录 + 8].copy_from_slice(&表校验.to_be_bytes());
    let head偏移 = head偏移.ok_or_else(|| 工具错误::新("字体缺少 head 表"))?;
    let 调整位置 = head偏移 + 8;
    字节
        .get_mut(调整位置..调整位置 + 4)
        .ok_or_else(|| 工具错误::新("head 表超出字体范围"))?
        .fill(0);
    let 总校验 = 表校验和(字节);
    let 调整 = 0xB1B0_AFBAu32.wrapping_sub(总校验);
    字节[调整位置..调整位置 + 4].copy_from_slice(&调整.to_be_bytes());
    Ok(())
}

/// SFNT 校验和：按 4 字节大端累加，尾部补零。
pub(super) fn 表校验和(字节: &[u8]) -> u32 {
    let mut 和 = 0u32;
    let mut 序号 = 0;
    while 序号 < 字节.len() {
        let mut 词 = [0u8; 4];
        let 剩余 = (字节.len() - 序号).min(4);
        词[..剩余].copy_from_slice(&字节[序号..序号 + 剩余]);
        和 = 和.wrapping_add(u32::from_be_bytes(词));
        序号 += 4;
    }
    和
}

/// 计算可省略尾部等宽度量的 numberOfHMetrics。
pub(super) fn 压缩度量数(前进列表: &[u16]) -> usize {
    let 末前进 = 前进列表.last().copied().unwrap_or(0);
    let mut 度量数 = 前进列表.len();
    while 度量数 > 1 && 前进列表[度量数 - 2] == 末前进 {
        度量数 -= 1;
    }
    度量数
}

/// 合并轮廓为单个 BezPath。
pub(super) fn 合并路径(轮廓: &[子路径]) -> BezPath {
    let mut 路径 = BezPath::new();
    for 项 in 轮廓 {
        路径.extend(项.转bezipath());
    }
    路径
}

/// 构建 cmap format 4 子表。
pub(super) fn 构建cmap4(码点表: &[(u16, usize)]) -> CmapSubtable {
    let mut 排序: Vec<(u16, usize)> = 码点表.to_vec();
    排序.sort_unstable();
    let mut 结束码 = Vec::new();
    let mut 起始码 = Vec::new();
    let mut 差值 = Vec::new();
    let mut 范围偏移 = Vec::new();
    let mut 序号 = 0;
    while 序号 < 排序.len() {
        let 起点 = 排序[序号].0;
        let 首字形 = 排序[序号].1;
        let mut 终点 = 起点;
        let mut 下一 = 序号 + 1;
        while 下一 < 排序.len()
            && 排序[下一].0 == 终点.wrapping_add(1)
            && 排序[下一].1 == 首字形 + (终点 - 起点) as usize + 1
        {
            终点 = 排序[下一].0;
            下一 += 1;
        }
        起始码.push(起点);
        结束码.push(终点);
        差值.push((首字形 as i64 - 起点 as i64) as i16);
        范围偏移.push(0);
        序号 = 下一;
    }
    起始码.push(0xFFFF);
    结束码.push(0xFFFF);
    差值.push(1);
    范围偏移.push(0);
    CmapSubtable::format_4(0, 结束码, 起始码, 差值, 范围偏移, Vec::new())
}
