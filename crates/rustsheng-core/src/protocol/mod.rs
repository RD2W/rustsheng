//! UV-K5 wire protocol: CRC, obfuscation, framing, and command builders.

pub mod crc;
pub mod frame;
pub mod obfuscation;

pub use crc::crc16_xmodem;
pub use frame::{ProtocolError, deframe, frame};
pub use obfuscation::{FIRMWARE_XOR, PAYLOAD_XOR, xor_firmware, xor_payload};

pub mod commands;

pub use commands::SESSION_ID;
