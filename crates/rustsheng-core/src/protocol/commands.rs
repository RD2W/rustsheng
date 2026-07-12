//! Builders for the clear command payloads sent to the radio. Each returns the
//! bytes that go *inside* a datagram (see [`crate::protocol::frame`]).
//!
//! Byte layouts are taken verbatim from the reference `k5prog.c` and
//! `k5prog-win/Unit1.cpp`.

/// Session identifier echoed in most commands; constant within a session.
pub const SESSION_ID: [u8; 4] = [0x6a, 0x39, 0x57, 0x64];

/// Flash block size (256 bytes); flash blocks are always padded to this size.
pub const FLASH_BLOCK: usize = 0x100;

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

/// `0x30` present the firmware version to the bootloader (flash mode only).
/// The version occupies a fixed 16-byte, zero-padded field.
pub fn flash_version(version: &str) -> Vec<u8> {
    let mut p = vec![0x30, 0x05, 0x10, 0x00];
    let mut field = [0u8; 16];
    let bytes = version.as_bytes();
    let n = bytes.len().min(16);
    field[..n].copy_from_slice(&bytes[..n]);
    p.extend_from_slice(&field);
    p
}

/// `0x19` write one flash block (flash mode only). The block offset is
/// **big-endian**; data is padded with zeros to [`FLASH_BLOCK`].
pub fn write_flash(offset: u16, data: &[u8], firmware_size: usize) -> Vec<u8> {
    let max_block_addr: u16 = if firmware_size & 0xff != 0 {
        ((firmware_size & 0xff00) + FLASH_BLOCK) as u16
    } else {
        (firmware_size & 0xff00) as u16
    };
    let inner = 12 + FLASH_BLOCK; // 0x10c
    let len = data.len();
    let mut p = vec![
        0x19,
        0x05,
        (inner & 0xff) as u8,
        ((inner >> 8) & 0xff) as u8,
        0x8a,
        0x8d,
        0x9f,
        0x1d,
        (offset >> 8) as u8,
        (offset & 0xff) as u8,
        (max_block_addr >> 8) as u8,
        0x00,
        (len & 0xff) as u8,
        ((len >> 8) & 0xff) as u8,
        0x00,
        0x00,
    ];
    p.extend_from_slice(data);
    p.resize(16 + FLASH_BLOCK, 0); // zero-pad data to a full block
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_matches_reference() {
        assert_eq!(hello(), vec![0x14, 0x05, 0x04, 0x00, 0x6a, 0x39, 0x57, 0x64]);
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

    #[test]
    fn write_flash_pads_to_full_block_and_is_big_endian() {
        let p = write_flash(0x0100, &[0x11, 0x22], 0x0800);
        assert_eq!(p.len(), 16 + FLASH_BLOCK);
        assert_eq!(&p[0..4], &[0x19, 0x05, 0x0c, 0x01]);
        assert_eq!(&p[4..8], &[0x8a, 0x8d, 0x9f, 0x1d]);
        assert_eq!(&p[8..10], &[0x01, 0x00]); // offset 0x0100 big-endian
        assert_eq!(&p[12..14], &[0x02, 0x00]); // len = 2, little-endian
        assert_eq!(p[16], 0x11);
        assert_eq!(p[17], 0x22);
        assert_eq!(p[18], 0x00); // padding
    }
}
