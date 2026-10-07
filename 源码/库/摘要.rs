// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 最小 SHA-256 实现，用于基准指纹；仅比较与展示，不用于安全用途。

const 初始状态: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const 轮常量: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// 计算 SHA-256 摘要。
#[must_use]
pub fn sha256(数据: &[u8]) -> [u8; 32] {
    let mut 消息 = 数据.to_vec();
    let 位长 = (数据.len() as u64) * 8;
    消息.push(0x80);
    while 消息.len() % 64 != 56 {
        消息.push(0);
    }
    消息.extend_from_slice(&位长.to_be_bytes());

    let mut 状态 = 初始状态;
    for 块 in 消息.as_chunks::<64>().0 {
        let mut 字 = [0u32; 64];
        for (序号, 字节) in 块.as_chunks::<4>().0.iter().enumerate() {
            字[序号] = u32::from_be_bytes([字节[0], 字节[1], 字节[2], 字节[3]]);
        }
        for 序号 in 16..64 {
            let s0 = 字[序号 - 15].rotate_right(7)
                ^ 字[序号 - 15].rotate_right(18)
                ^ (字[序号 - 15] >> 3);
            let s1 = 字[序号 - 2].rotate_right(17)
                ^ 字[序号 - 2].rotate_right(19)
                ^ (字[序号 - 2] >> 10);
            字[序号] = 字[序号 - 16]
                .wrapping_add(s0)
                .wrapping_add(字[序号 - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = 状态;
        for 序号 in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let 临时1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(轮常量[序号])
                .wrapping_add(字[序号]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let 临时2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(临时1);
            d = c;
            c = b;
            b = a;
            a = 临时1.wrapping_add(临时2);
        }
        状态[0] = 状态[0].wrapping_add(a);
        状态[1] = 状态[1].wrapping_add(b);
        状态[2] = 状态[2].wrapping_add(c);
        状态[3] = 状态[3].wrapping_add(d);
        状态[4] = 状态[4].wrapping_add(e);
        状态[5] = 状态[5].wrapping_add(f);
        状态[6] = 状态[6].wrapping_add(g);
        状态[7] = 状态[7].wrapping_add(h);
    }
    let mut 结果 = [0u8; 32];
    for (序号, 值) in 状态.iter().enumerate() {
        结果[序号 * 4..序号 * 4 + 4].copy_from_slice(&值.to_be_bytes());
    }
    结果
}

/// 计算小写十六进制摘要。
#[must_use]
pub fn sha256十六进制(数据: &[u8]) -> String {
    sha256(数据)
        .iter()
        .fold(String::with_capacity(64), |mut 结果, 字节| {
            结果.push_str(&format!("{字节:02x}"));
            结果
        })
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 标准测试向量() {
        assert_eq!(
            sha256十六进制(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256十六进制(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256十六进制(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }
}
