// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
//! 复用 RustCrypto SHA-256，保持图标来源与基准指纹的摘要接口。

use sha2::{Digest, Sha256};

/// 计算固定32字节的SHA-256摘要，不复制整份输入用于填充。
#[must_use]
pub fn sha256(数据: &[u8]) -> [u8; 32] {
    Sha256::digest(数据).into()
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
    #[test]
    fn 二进制分块边界() {
        // 期望值由独立Python hashlib生成，覆盖旧实现的填充与分块边界。
        for (长度, 预期) in [
            (
                55,
                "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59",
            ),
            (
                56,
                "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562",
            ),
            (
                63,
                "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488",
            ),
            (
                64,
                "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108",
            ),
            (
                65,
                "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781",
            ),
            (
                127,
                "92ca0fa6651ee2f97b884b7246a562fa71250fedefe5ebf270d31c546bfea976",
            ),
            (
                128,
                "471fb943aa23c511f6f72f8d1652d9c880cfa392ad80503120547703e56a2be5",
            ),
        ] {
            let 数据: Vec<u8> = (0..长度).map(|值| (值 % 256) as u8).collect();
            assert_eq!(sha256十六进制(&数据), 预期);
        }
    }
}
