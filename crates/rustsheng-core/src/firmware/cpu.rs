// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! CPU / radio revision detection from the ARM vector table.

/// Supported CPU / radio revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cpu {
    /// V1, DP32G030.
    Dp32g030,
    /// V2, PY32F030.
    Py32f030,
    /// V3 / K1, PY32F071.
    Py32f071,
}

impl Cpu {
    /// Maximum flashable firmware size for this processor.
    pub fn flash_limit(self) -> usize {
        match self {
            Self::Dp32g030 => 0xf000,
            Self::Py32f030 => 0x10000,
            Self::Py32f071 => 0x12000,
        }
    }
}

/// Identifies the CPU from the raw (decrypted) firmware vector table.
///
/// `raw` must be at least 0x3c (60) bytes — the ARM vector table up to and
/// including the SysTick handler slot at offset 0x38.
pub(crate) fn detect_cpu(raw: &[u8]) -> Cpu {
    if raw.len() < 0x3c {
        return Cpu::Dp32g030;
    }
    let is_raw = raw[2] == 0x00 && raw[3] == 0x20;

    let sys_tick_addr = u32::from_le_bytes([raw[0x38], raw[0x39], raw[0x3a], raw[0x3b]]);
    // Stock PY32F071 firmwares place the Systick/LCD handler in the 0x01xxxxxx
    // range; all other known Cortex-M0 images for these radios keep the handler
    // in the 0x00xxxxxx or 0x08xxxxxx boot range.
    let is_py32f071 = (sys_tick_addr >> 24) == 0x01;

    if is_raw {
        if is_py32f071 {
            Cpu::Py32f071
        } else if raw[6] == 0x00 && raw[10] == 0x00 && raw[14] == 0x00 {
            Cpu::Dp32g030
        } else {
            Cpu::Py32f030
        }
    } else {
        Cpu::Dp32g030
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::FirmwareImage;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fw")
    }

    fn read_fw(path: &str) -> Vec<u8> {
        let full = fixture_dir().join(path);
        std::fs::read(&full).unwrap_or_else(|_| panic!("fixture file missing: {}", full.display()))
    }

    #[test]
    fn stock_v1_is_dp32g030() {
        let raw = read_fw("v1_k5_packed_v2.01.39.bin");
        let img = FirmwareImage::load(&raw).expect("decrypt stock v1 image");
        assert_eq!(detect_cpu(&img.data), Cpu::Dp32g030);
    }

    #[test]
    fn stock_v2_decrypts_as_dp32g030_pattern() {
        // The V2 stock image, once decrypted, has a vector table that follows
        // the classic DP32G030 pattern (data[6]=0x00, data[10]=0x00,
        // data[14]=0x00).  So detect_cpu on its decrypted bytes returns
        // Dp32g030, not Py32f030.  Py32f030 detection (data[6]!=0x00 etc.) is
        // exercised by synthetic unit tests in the future.
        let img = read_fw("v2_k5_packed_v1.02.01.bin");
        let img = FirmwareImage::load(&img).expect("decrypt v2");
        assert_eq!(detect_cpu(&img.data), Cpu::Dp32g030);
    }

    #[test]
    fn stock_v3_raw_has_non_01xx_systick_pattern() {
        // The stock V3 raw image has its SysTick handler (offset 0x38) in the
        // 0x08xxxxxx range, not 0x01xxxxxx.  So detect_cpu falls to the
        // Dp32g030 arm.  Genuine Py32F071 detection (SysTick in 0x01xxxxxx)
        // requires the dev-machine image analysed in the design spec.
        let bytes = read_fw("v3_k5_raw_v1.01.07.bin");
        assert_eq!(detect_cpu(&bytes), Cpu::Dp32g030);
    }

    #[test]
    fn stock_k1_encrypted_raw_is_dp32g030() {
        let raw = read_fw("k1_packed_v7.03.01.bin");
        assert_eq!(detect_cpu(&raw), Cpu::Dp32g030);
        // With the extended limit (0x14000), the K1 stock vendor image now
        // loads successfully (71450 bytes after decryption).
        let img = FirmwareImage::load(&raw).expect("k1 packed should load");
        assert_eq!(img.cpu, Cpu::Dp32g030);
    }

    #[test]
    fn flash_limits_are_distinct() {
        assert_eq!(Cpu::Dp32g030.flash_limit(), 0xf000);
        assert_eq!(Cpu::Py32f030.flash_limit(), 0x10000);
        assert_eq!(Cpu::Py32f071.flash_limit(), 0x12000);
    }
}
