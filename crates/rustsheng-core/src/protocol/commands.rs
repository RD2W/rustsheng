// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Builders for the clear command payloads sent to the radio. Each returns the
//! bytes that go *inside* a datagram (see [`crate::protocol::frame`]).
//!
//! Byte layouts are taken verbatim from the reference `k5prog.c` and
//! `k5prog-win/Unit1.cpp`.

/// Session identifier echoed in most commands; constant within a session.
pub const SESSION_ID: [u8; 4] = [0x6a, 0x39, 0x57, 0x64];

/// `0x14` handshake ("hello").
pub fn hello() -> Vec<u8> {
    let mut p = vec![0x14, 0x05, 0x04, 0x00];
    p.extend_from_slice(&SESSION_ID);
    p
}

/// `0x1b` read EEPROM block (`len` <= 0x80). Address is little-endian.
pub fn read_eeprom(addr: u16, len: u8) -> Vec<u8> {
    let mut p = vec![
        0x1b,
        0x05,
        0x08,
        0x00,
        (addr & 0xff) as u8,
        (addr >> 8) as u8,
        len,
        0x00,
    ];
    p.extend_from_slice(&SESSION_ID);
    p
}

/// `0x1d` write EEPROM block (`data.len()` <= 0x80). Address is little-endian.
pub fn write_eeprom(addr: u16, data: &[u8]) -> Vec<u8> {
    let inner = 8 + data.len();
    let mut p = vec![
        0x1d,
        0x05,
        (inner & 0xff) as u8,
        ((inner >> 8) & 0xff) as u8,
        (addr & 0xff) as u8,
        (addr >> 8) as u8,
        data.len() as u8,
        0x01,
    ];
    p.extend_from_slice(&SESSION_ID);
    p.extend_from_slice(data);
    p
}

/// `0xdd` reboot the radio.
pub fn reset() -> Vec<u8> {
    vec![0xdd, 0x05, 0x00, 0x00]
}

/// `0x29` read battery ADC.
pub fn read_adc() -> Vec<u8> {
    let mut p = vec![0x29, 0x05, 0x04, 0x00];
    p.extend_from_slice(&SESSION_ID);
    p
}

/// `0x27` read RSSI/noise/glitch.
pub fn read_rssi() -> Vec<u8> {
    let mut p = vec![0x27, 0x05, 0x04, 0x00];
    p.extend_from_slice(&SESSION_ID);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_matches_reference() {
        assert_eq!(
            hello(),
            vec![0x14, 0x05, 0x04, 0x00, 0x6a, 0x39, 0x57, 0x64]
        );
    }

    #[test]
    fn read_eeprom_encodes_little_endian_address() {
        let p = read_eeprom(0x0e80, 0x80);
        assert_eq!(&p[0..8], &[0x1b, 0x05, 0x08, 0x00, 0x80, 0x0e, 0x80, 0x00]);
    }

    #[test]
    fn write_eeprom_sets_length_fields() {
        let data = [0xAA; 0x10];
        let p = write_eeprom(0x0010, &data);
        assert_eq!(&p[0..8], &[0x1d, 0x05, 0x18, 0x00, 0x10, 0x00, 0x10, 0x01]);
        assert_eq!(p.len(), 8 + 4 + 0x10);
    }
}
