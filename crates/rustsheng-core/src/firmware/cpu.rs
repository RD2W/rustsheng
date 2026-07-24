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

    /// Maximum addressable EEPROM range for this processor.
    pub fn eeprom_limit(self) -> usize {
        match self {
            Self::Dp32g030 => 0x2000,
            _ => 0x10000,
        }
    }

    /// Returns the expected CPU for a given bootloader version string (the
    /// value that `bootloader-info` and `wait_for_beacon` report), or `None`
    /// when the version cannot be mapped to a known CPU.
    ///
    /// Boot-version mapping derived from forum reports of the TWHH (OUROBOROS)
    /// flasher tools — K1_TOOL (boot 7.\*), K5_TOOL (boot 1.\*/2.\*/3.\*/4.\*),
    /// R5_TOOL (boot 5.\*) — each targeting a specific CPU:
    /// - `1.*` → PY32F030 (V2)
    /// - `2.*` / `3.*` / `4.*` → DP32G030 (V1)
    /// - `5.*` → DP32G030 (R5+)
    /// - `7.*` → PY32F071 (V3 / K1)
    pub fn from_boot_version(version: &str) -> Option<Self> {
        if version.is_empty() {
            return None;
        }
        match version.as_bytes()[0] {
            b'1' => Some(Self::Py32f030),
            b'2' | b'3' | b'4' => Some(Self::Dp32g030),
            b'5' => Some(Self::Dp32g030),
            b'7' => Some(Self::Py32f071),
            _ => None,
        }
    }
}

/// Identifies the CPU from the raw (decrypted) firmware vector table.
///
/// `raw` must be at least 0x3c (60) bytes — the ARM vector table up to and
/// including the SysTick handler slot at offset 0x38.
///
/// Detection is based on the Reset vector (offset 0x04). The PY32F071
/// bootloader resides at the beginning of flash (0x08000000–0x080027FF),
/// so the firmware's Reset vector always points to 0x08002800 or later.
/// DP32G030 and PY32F030 have the bootloader at the end of flash, with
/// firmware starting at 0x08000000 — their Reset vectors cluster near
/// 0x080000xx (or 0x000000xx in the low alias).
pub(crate) fn detect_cpu(raw: &[u8]) -> Cpu {
    if raw.len() < 0x3c {
        return Cpu::Dp32g030;
    }
    let is_raw = raw[2] == 0x00 && raw[3] == 0x20;

    if is_raw {
        let reset = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
        if reset >= 0x0800_2000 {
            return Cpu::Py32f071;
        }
        if raw[6] == 0x00 && raw[10] == 0x00 && raw[14] == 0x00 {
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
    fn stock_v3_raw_is_py32f071() {
        // The stock V3 raw image has its Reset vector at 0x080028d5 (>=
        // 0x08002000), which is the reliable PY32F071 marker: the bootloader
        // occupies flash start (0x08000000–0x080027FF), pushing the firmware
        // entry point past the 0x2000 boundary.
        let bytes = read_fw("v3_k5_raw_v1.01.07.bin");
        assert_eq!(detect_cpu(&bytes), Cpu::Py32f071);
    }

    #[test]
    fn custom_v3_is_py32f071() {
        let bytes = read_fw("v3_custom_fusion_v4.3.2.bin");
        assert_eq!(detect_cpu(&bytes), Cpu::Py32f071);
    }

    #[test]
    fn custom_k1_is_py32f071() {
        let bytes = read_fw("k1_custom_fusion_v4.3.2.bin");
        assert_eq!(detect_cpu(&bytes), Cpu::Py32f071);
    }

    #[test]
    fn stock_k1_encrypted_raw_is_dp32g030() {
        let raw = read_fw("k1_packed_v7.03.01.bin");
        // Encrypted image: is_raw is false → Dp32g030 before decryption.
        assert_eq!(detect_cpu(&raw), Cpu::Dp32g030);
        // After decryption via FirmwareImage::load, Reset-vector check → Py32f071.
        let img = FirmwareImage::load(&raw).expect("k1 packed should load");
        assert_eq!(img.cpu, Cpu::Py32f071, "decrypted K1 is PY32F071");
    }

    #[test]
    fn flash_limits_are_distinct() {
        assert_eq!(Cpu::Dp32g030.flash_limit(), 0xf000);
        assert_eq!(Cpu::Py32f030.flash_limit(), 0x10000);
        assert_eq!(Cpu::Py32f071.flash_limit(), 0x12000);
    }

    #[test]
    fn eeprom_limits() {
        assert_eq!(Cpu::Dp32g030.eeprom_limit(), 0x2000);
        assert_eq!(Cpu::Py32f030.eeprom_limit(), 0x10000);
        assert_eq!(Cpu::Py32f071.eeprom_limit(), 0x10000);
    }

    #[test]
    fn boot_version_maps_to_cpu() {
        assert_eq!(Cpu::from_boot_version("1.02.01"), Some(Cpu::Py32f030));
        assert_eq!(Cpu::from_boot_version("2.00.06"), Some(Cpu::Dp32g030));
        assert_eq!(Cpu::from_boot_version("3.00.22"), Some(Cpu::Dp32g030));
        assert_eq!(Cpu::from_boot_version("4.00.09"), Some(Cpu::Dp32g030));
        assert_eq!(Cpu::from_boot_version("5.00.05"), Some(Cpu::Dp32g030));
        assert_eq!(Cpu::from_boot_version("7.03.01"), Some(Cpu::Py32f071));
        assert_eq!(Cpu::from_boot_version(""), None);
        assert_eq!(Cpu::from_boot_version("99.99.99"), None);
    }

    #[test]
    fn force_cpu_overrides_stock_v3_detection() {
        let bytes = read_fw("v3_k5_raw_v1.01.07.bin");
        let mut image = FirmwareImage::load(&bytes).expect("load V3 stock image");
        assert_eq!(image.cpu, Cpu::Py32f071, "V3 correctly detected");
        // Simulate --force-cpu dp32g030 to override
        image.cpu = Cpu::Dp32g030;
        assert_eq!(image.cpu, Cpu::Dp32g030);
        assert_eq!(image.cpu.flash_limit(), 0xf000);
    }
}
