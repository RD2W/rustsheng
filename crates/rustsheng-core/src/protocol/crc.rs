// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF) and k5prog-win (OneOfEleven).

//! CRC-16/XMODEM used by the UV-K5 datagram format.

/// Computes CRC-16/XMODEM (polynomial `0x1021`, initial value `0x0000`,
/// no reflection, no final XOR) over `data`.
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_value() {
        // The standard CRC-16/XMODEM check value for the ASCII string
        // "123456789" is 0x31C3.
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(crc16_xmodem(&[]), 0x0000);
    }
}
