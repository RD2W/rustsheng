//! UV-K5 wire protocol: CRC, obfuscation, framing, and command builders.

pub mod crc;
pub mod obfuscation;

pub use crc::crc16_xmodem;
pub use obfuscation::{FIRMWARE_XOR, PAYLOAD_XOR, xor_firmware, xor_payload};
