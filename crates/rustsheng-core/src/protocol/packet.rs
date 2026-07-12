// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Human-readable description of a clear datagram payload, keyed by its 16-bit
//! command identifier (the first two payload bytes, little-endian). Used by the
//! `parse` and `sniffer` commands and by verbose diagnostics.

/// Returns the printable ASCII prefix of `bytes`, stopping at the first NUL or
/// non-printable byte.
fn ascii_prefix(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&b| b == 0 || !b.is_ascii_graphic() && b != b' ')
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Describes a clear payload as `Name (0xID)` plus a few decoded fields for the
/// common commands. Unknown identifiers are reported as `Unknown (0xID)`.
pub fn describe(payload: &[u8]) -> String {
    if payload.len() < 2 {
        return format!("<short payload, {} bytes>", payload.len());
    }
    let id = u16::from_le_bytes([payload[0], payload[1]]);
    let name = match id {
        0x0514 => "HelloReq",
        0x0515 => "HelloAck",
        0x0518 => "FlashBeacon",
        0x0519 => "FlashWriteReq",
        0x051a => "FlashWriteAck",
        0x051b => "ReadEepromReq",
        0x051c => "ReadEepromAck",
        0x051d => "WriteEepromReq",
        0x051e => "WriteEepromAck",
        0x0527 => "ReadRssiReq",
        0x0528 => "ReadRssiAck",
        0x0529 => "ReadAdcReq",
        0x052a => "ReadAdcAck",
        0x0530 => "FlashVersionReq",
        0x05dd => "RebootReq",
        0x057a => "Flash5Beacon",
        0x057b => "Flash5WriteReq",
        _ => "Unknown",
    };

    let extra = match id {
        // Firmware version is a 16-byte field at offset 4.
        0x0515 if payload.len() >= 20 => format!(" version={:?}", ascii_prefix(&payload[4..20])),
        // Bootloader version starts at offset 0x14 in the flash beacon.
        0x0518 if payload.len() > 0x14 => format!(" version={:?}", ascii_prefix(&payload[0x14..])),
        // EEPROM commands carry a little-endian address at offset 4.
        0x051b..=0x051e if payload.len() >= 7 => {
            format!(
                " offset={:#06x} len={:#04x}",
                u16::from_le_bytes([payload[4], payload[5]]),
                payload[6]
            )
        }
        _ => String::new(),
    };

    format!("{name} ({id:#06x}){extra}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_hello_request() {
        let d = describe(&[0x14, 0x05, 0x04, 0x00, 0x6a, 0x39, 0x57, 0x64]);
        assert!(d.contains("HelloReq"), "{d}");
        assert!(d.contains("0x0514"), "{d}");
    }

    #[test]
    fn decodes_read_eeprom_offset() {
        let d = describe(&[0x1b, 0x05, 0x08, 0x00, 0x80, 0x0e, 0x80, 0x00]);
        assert!(d.contains("ReadEepromReq"), "{d}");
        assert!(d.contains("offset=0x0e80"), "{d}");
        assert!(d.contains("len=0x80"), "{d}");
    }

    #[test]
    fn extracts_hello_ack_version() {
        let mut p = vec![0x15u8, 0x05, 0x24, 0x00];
        let mut ver = [0u8; 16];
        ver[..10].copy_from_slice(b"k5_2.01.26");
        p.extend_from_slice(&ver);
        let d = describe(&p);
        assert!(d.contains("HelloAck"), "{d}");
        assert!(d.contains("k5_2.01.26"), "{d}");
    }

    #[test]
    fn reports_unknown() {
        let d = describe(&[0xff, 0x05]);
        assert!(d.contains("Unknown"), "{d}");
        assert!(d.contains("0x05ff"), "{d}");
    }
}
