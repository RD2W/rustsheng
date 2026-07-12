// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Firmware flashing protocols (V2 unencrypted, V5 AES-CBC-128).

pub mod v2;
#[cfg(feature = "flash-v5")]
pub mod v5;

/// Flash write block size.
pub const FLASH_BLOCK: usize = 0x100;
/// Hard upper bound for a flashable image.
pub const MAX_FLASH: usize = 0xf000;
/// Fixed write-request id (K5TOOL randomizes it; we pin it for determinism).
pub const WRITE_ID: u32 = 0x1d9f8d8a;

/// Which bootloader flash protocol a radio speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashKind {
    /// Unencrypted (bootloader beacon 0x0518).
    V2,
    /// AES-CBC-128 encrypted (bootloader beacon 0x057a).
    V5,
}

/// Errors from the flash layer.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FlashError {
    /// A V5 radio was detected but the crate was built without `flash-v5`.
    #[error("V5 flashing requires the `flash-v5` feature")]
    V5Unavailable,
    /// Image exceeds MAX_FLASH.
    #[error("firmware image too large (max {MAX_FLASH:#x})")]
    TooLarge,
}

/// A bootloader flash protocol (V2 or V5).
pub trait FlashProtocol {
    /// Beacon datagram id that selects this protocol.
    fn beacon_id(&self) -> u16;
    /// Write-ack datagram id.
    fn ack_id(&self) -> u16;
    /// Clear payload of the version request.
    fn version_packet(&self, version: &str) -> Vec<u8>;
    /// Called once before the first block (V5 initializes the AES stream).
    fn begin(&mut self);
    /// Clear payload of a write request for one 0xff-padded block.
    fn write_packet(
        &mut self,
        chunk_no: u16,
        chunk_count: u16,
        block: &[u8; FLASH_BLOCK],
        len: u16,
        id: u32,
    ) -> Vec<u8>;
    /// Parses a write ack into `(chunk_no, result)`, or `None` if not an ack.
    fn parse_write_ack(&self, payload: &[u8]) -> Option<(u16, u8)> {
        let cmd = (self.ack_id() & 0xff) as u8;
        if payload.len() >= 11 && payload[0] == cmd && payload[1] == 0x05 {
            Some((u16::from_le_bytes([payload[8], payload[9]]), payload[10]))
        } else {
            None
        }
    }
}

/// Builds the clear payload of a write request. `cmd` is `0x19` (V2) or `0x7b`
/// (V5); `block` is the already-0xff-padded (and, for V5, already-encrypted)
/// 0x100-byte page.
pub(crate) fn make_write_payload(
    cmd: u8,
    chunk_no: u16,
    chunk_count: u16,
    block: &[u8; FLASH_BLOCK],
    len: u16,
    id: u32,
) -> Vec<u8> {
    let mut p = Vec::with_capacity(16 + FLASH_BLOCK);
    p.push(cmd);
    p.push(0x05);
    p.push(0x0c); // hdrSize 0x010c, LE
    p.push(0x01);
    p.extend_from_slice(&id.to_le_bytes()); // 8a 8d 9f 1d
    p.extend_from_slice(&chunk_no.to_le_bytes());
    p.extend_from_slice(&chunk_count.to_le_bytes());
    p.extend_from_slice(&len.to_le_bytes());
    p.push(0x00);
    p.push(0x00); // padding
    p.extend_from_slice(block);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_payload_layout() {
        let block = [0xffu8; FLASH_BLOCK];
        let p = make_write_payload(0x19, 0x0002, 0x00ef, &block, 0x100, WRITE_ID);
        assert_eq!(p.len(), 16 + FLASH_BLOCK);
        assert_eq!(&p[0..8], &[0x19, 0x05, 0x0c, 0x01, 0x8a, 0x8d, 0x9f, 0x1d]);
        assert_eq!(&p[8..10], &[0x02, 0x00]); // chunk_no LE
        assert_eq!(&p[10..12], &[0xef, 0x00]); // chunk_count LE
        assert_eq!(&p[12..14], &[0x00, 0x01]); // len 0x100 LE
        assert_eq!(&p[14..16], &[0x00, 0x00]); // padding
        assert_eq!(p[16], 0xff);
    }
}
