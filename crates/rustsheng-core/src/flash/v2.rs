// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! V2 (unencrypted) flash protocol.

use super::{FLASH_BLOCK, FlashProtocol, make_write_payload};

/// Unencrypted flash protocol (bootloader beacon `0x0518`).
#[derive(Debug, Default)]
pub struct ProtocolV2;

impl ProtocolV2 {
    /// Creates a V2 protocol handler.
    pub fn new() -> Self {
        Self
    }
}

impl FlashProtocol for ProtocolV2 {
    fn beacon_id(&self) -> u16 {
        0x0518
    }

    fn ack_id(&self) -> u16 {
        0x051a
    }

    fn version_packet(&self, version: &str) -> Vec<u8> {
        // 0x0530: 30 05 10 00 + 16 zero-padded version bytes.
        let mut p = vec![0x30, 0x05, 0x10, 0x00];
        let mut field = [0u8; 16];
        let bytes = version.as_bytes();
        let n = bytes.len().min(16);
        field[..n].copy_from_slice(&bytes[..n]);
        p.extend_from_slice(&field);
        p
    }

    fn begin(&mut self) {}

    fn write_packet(
        &mut self,
        chunk_no: u16,
        chunk_count: u16,
        block: &[u8; FLASH_BLOCK],
        len: u16,
        id: u32,
    ) -> Vec<u8> {
        make_write_payload(0x19, chunk_no, chunk_count, block, len, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flash::WRITE_ID;

    #[test]
    fn version_packet_layout() {
        let p = ProtocolV2::new().version_packet("2.01.23");
        assert_eq!(&p[0..4], &[0x30, 0x05, 0x10, 0x00]);
        assert_eq!(&p[4..11], b"2.01.23");
        assert_eq!(p[11], 0x00);
        assert_eq!(p.len(), 20);
    }

    #[test]
    fn write_packet_uses_0x19() {
        let block = [0u8; FLASH_BLOCK];
        let p = ProtocolV2::new().write_packet(1, 2, &block, 0x100, WRITE_ID);
        assert_eq!(p[0], 0x19);
        assert_eq!(&p[8..12], &[0x01, 0x00, 0x02, 0x00]);
    }

    #[test]
    fn parses_write_ack() {
        // 1a 05 08 00 8a 8d 9f 1d chunk(2) result(1) v0(1)
        let ack = [
            0x1a, 0x05, 0x08, 0x00, 0x8a, 0x8d, 0x9f, 0x1d, 0x03, 0x00, 0x00, 0x00,
        ];
        assert_eq!(ProtocolV2::new().parse_write_ack(&ack), Some((3, 0)));
    }
}
