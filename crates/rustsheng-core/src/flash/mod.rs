// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Firmware flashing protocols (V2 unencrypted, V5 AES-CBC-128).

pub mod v2;
#[cfg(feature = "flash-v5")]
pub mod v5;

use crate::firmware::FirmwareImage;
use crate::protocol::frame;
use v2::ProtocolV2;

/// Flash write block size.
pub const FLASH_BLOCK: usize = 0x100;
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
    /// Image exceeds the maximum flashable size for its CPU.
    #[error("firmware image too large for CPU")]
    TooLarge,
}

/// A bootloader flash protocol (V2 or V5).
pub trait FlashProtocol {
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
        if payload.len() >= 11 && u16::from_le_bytes([payload[0], payload[1]]) == self.ack_id() {
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

/// Yields `(chunk_no, chunk_count, padded_block, len)` for each FLASH_BLOCK-sized
/// page of `data` (last page 0xff-padded).
pub(crate) fn blocks(data: &[u8]) -> impl Iterator<Item = (u16, u16, [u8; FLASH_BLOCK], u16)> + '_ {
    let chunk_count = data.len().div_ceil(FLASH_BLOCK) as u16;
    data.chunks(FLASH_BLOCK).enumerate().map(move |(i, chunk)| {
        let mut block = [0xffu8; FLASH_BLOCK];
        block[..chunk.len()].copy_from_slice(chunk);
        (i as u16, chunk_count, block, chunk.len() as u16)
    })
}

/// Builds the ordered, framed datagram stream for flashing `image`: the version
/// request followed by one write request per 0x100 block. Pure (no I/O); used by
/// `--dry-run` and by tests.
pub fn build_sequence(
    kind: FlashKind,
    image: &FirmwareImage,
    version: &str,
    key_number: u8,
    id: u32,
) -> Result<Vec<Vec<u8>>, FlashError> {
    let mut proto: Box<dyn FlashProtocol> = match kind {
        FlashKind::V2 => Box::new(ProtocolV2::new()),
        FlashKind::V5 => {
            #[cfg(feature = "flash-v5")]
            {
                Box::new(v5::ProtocolV5::new(key_number))
            }
            #[cfg(not(feature = "flash-v5"))]
            {
                let _ = key_number;
                return Err(FlashError::V5Unavailable);
            }
        }
    };

    let mut out = Vec::new();
    out.push(frame(&proto.version_packet(version)));
    proto.begin();

    for (chunk_no, chunk_count, block, len) in blocks(&image.data) {
        out.push(frame(&proto.write_packet(
            chunk_no,
            chunk_count,
            &block,
            len,
            id,
        )));
    }
    Ok(out)
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

    #[test]
    fn build_sequence_v2_shapes() {
        use crate::firmware::FirmwareImage;
        let mut raw = vec![0u8; 0x800];
        raw[2] = 0x00;
        raw[3] = 0x20;
        raw[6] = 0x00;
        raw[10] = 0x00;
        raw[14] = 0x00;
        let img = FirmwareImage::load(&raw).unwrap();
        let seq = build_sequence(FlashKind::V2, &img, "2.01.23", 0, WRITE_ID).unwrap();
        assert_eq!(seq.len(), 1 + 0x800 / FLASH_BLOCK); // 1 version + 8 write packets
        for dg in &seq {
            assert_eq!(&dg[0..2], &[0xAB, 0xCD]);
            assert_eq!(&dg[dg.len() - 2..], &[0xDC, 0xBA]);
        }
    }

    #[cfg(feature = "flash-v5")]
    #[test]
    fn build_sequence_v5_shapes() {
        use crate::firmware::FirmwareImage;
        let mut raw = vec![0u8; 0x800];
        raw[2] = 0x00;
        raw[3] = 0x20;
        raw[6] = 0x00;
        raw[10] = 0x00;
        raw[14] = 0x00;
        let img = FirmwareImage::load(&raw).unwrap();
        let seq = build_sequence(FlashKind::V5, &img, "5.00.05", 0, WRITE_ID).unwrap();
        assert_eq!(seq.len(), 1 + 0x800 / FLASH_BLOCK);
        for dg in &seq {
            assert_eq!(&dg[0..2], &[0xAB, 0xCD]);
            assert_eq!(&dg[dg.len() - 2..], &[0xDC, 0xBA]);
        }
    }
}
