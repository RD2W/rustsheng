// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF) and k5prog-win (OneOfEleven).

//! Firmware image handling: detect raw vs vendor-encrypted images, decrypt when
//! needed, strip the embedded version, and split into flash blocks.

use log::debug;

use crate::protocol::{crc16_xmodem, xor_firmware};

/// Hard upper bound for a flashable image (the bootloader lives above this).
pub const MAX_FLASH: usize = 0xf000;
/// Flash write block size.
pub const BLOCK: usize = 0x100;
/// Version string accepted by all known bootloaders.
pub const DEFAULT_VERSION: &str = "*.01.23";

/// Errors from [`FirmwareImage::load`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FirmwareError {
    /// File is far too small to be firmware.
    #[error("firmware file too small")]
    TooSmall,
    /// Image exceeds [`MAX_FLASH`] and would run into the bootloader.
    #[error("firmware image too large (max {MAX_FLASH:#x})")]
    TooLarge,
    /// Neither a recognizable raw image nor a decryptable vendor image.
    #[error("file is not a valid firmware image")]
    Invalid,
}

/// A firmware image ready to flash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    /// Raw (decrypted, version-stripped) image bytes.
    pub data: Vec<u8>,
    /// Version string extracted from a vendor-encrypted image, if any.
    pub embedded_version: Option<String>,
}

/// Returns true if `data` looks like a raw DP32G030 image (ARM vector table).
fn looks_raw(data: &[u8]) -> bool {
    data.len() >= 15
        && data[2] == 0x00
        && data[3] == 0x20
        && data[6] == 0x00
        && data[10] == 0x00
        && data[14] == 0x00
}

impl FirmwareImage {
    /// Loads firmware from `bytes`, auto-detecting and decrypting a vendor image.
    pub fn load(bytes: &[u8]) -> Result<Self, FirmwareError> {
        if bytes.len() < 0x800 {
            return Err(FirmwareError::TooSmall);
        }

        // Already a raw image?
        if looks_raw(bytes) {
            let data = bytes.to_vec();
            if data.len() > MAX_FLASH {
                return Err(FirmwareError::TooLarge);
            }
            debug!("firmware: raw image ({} bytes)", data.len());
            return Ok(Self {
                data,
                embedded_version: None,
            });
        }

        // Otherwise it must be a vendor image: CRC-terminated and encrypted.
        let crc_want = crc16_xmodem(&bytes[..bytes.len() - 2]);
        let crc_got = u16::from_le_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]]);
        if crc_got != crc_want {
            return Err(FirmwareError::Invalid);
        }

        let mut data = bytes[..bytes.len() - 2].to_vec();
        xor_firmware(&mut data);
        if !looks_raw(&data) {
            return Err(FirmwareError::Invalid);
        }

        // Strip the 16-byte embedded version at 0x2000, if present.
        let mut embedded_version = None;
        if data.len() >= 0x2000 + 16 {
            let raw = &data[0x2000..0x2000 + 16];
            let end = raw.iter().position(|&b| b == 0).unwrap_or(16);
            embedded_version = Some(String::from_utf8_lossy(&raw[..end]).into_owned());
            data.drain(0x2000..0x2000 + 16);
        }
        debug!(
            "firmware: decrypted vendor image ({} bytes, version {:?})",
            data.len(),
            embedded_version.as_deref()
        );

        if data.len() > MAX_FLASH {
            return Err(FirmwareError::TooLarge);
        }
        Ok(Self {
            data,
            embedded_version,
        })
    }

    /// Splits the image into `(offset, chunk)` pairs of at most [`BLOCK`] bytes.
    pub fn blocks(&self) -> Vec<(u16, &[u8])> {
        self.data
            .chunks(BLOCK)
            .enumerate()
            .map(|(i, chunk)| ((i * BLOCK) as u16, chunk))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{crc16_xmodem, xor_firmware};

    fn raw_image(len: usize) -> Vec<u8> {
        let mut v = vec![0u8; len];
        // ARM vector-table markers checked by `looks_raw`.
        v[2] = 0x00;
        v[3] = 0x20;
        v[6] = 0x00;
        v[10] = 0x00;
        v[14] = 0x00;
        v
    }

    #[test]
    fn loads_raw_image() {
        let img = FirmwareImage::load(&raw_image(0x1000)).unwrap();
        assert_eq!(img.embedded_version, None);
        assert_eq!(img.data.len(), 0x1000);
    }

    #[test]
    fn rejects_tiny_file() {
        assert_eq!(
            FirmwareImage::load(&[0u8; 16]),
            Err(FirmwareError::TooSmall)
        );
    }

    #[test]
    fn decrypts_vendor_image_and_strips_version() {
        // Build a raw image with a version string at 0x2000, then encrypt it.
        let mut raw = raw_image(0x2000 + 16 + 0x40);
        raw[0x2000..0x2000 + 7].copy_from_slice(b"2.01.23");
        let mut enc = raw.clone();
        xor_firmware(&mut enc);
        let crc = crc16_xmodem(&enc);
        enc.push((crc & 0xff) as u8);
        enc.push((crc >> 8) as u8);

        let img = FirmwareImage::load(&enc).unwrap();
        assert_eq!(img.embedded_version.as_deref(), Some("2.01.23"));
        assert_eq!(img.data.len(), raw.len() - 16);
    }

    #[test]
    fn blocks_are_block_sized() {
        let img = FirmwareImage::load(&raw_image(0x850)).unwrap();
        let blocks = img.blocks();
        assert_eq!(blocks[0].0, 0x0000);
        assert_eq!(blocks[1].0, 0x0100);
        assert_eq!(blocks[8].0, 0x0800);
        assert_eq!(blocks[8].1.len(), 0x50);
    }
}
