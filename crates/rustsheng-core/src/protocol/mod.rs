//! UV-K5 wire protocol: CRC, obfuscation, framing, and command builders.

pub mod crc;

pub use crc::crc16_xmodem;
