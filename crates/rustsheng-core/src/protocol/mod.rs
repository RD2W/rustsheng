// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! UV-K5 wire protocol: CRC, obfuscation, framing, and command builders.

pub mod crc;
pub mod frame;
pub mod obfuscation;
pub mod packet;

pub use crc::crc16_xmodem;
pub use frame::{FrameScanner, ProtocolError, deframe, frame};
pub use obfuscation::{FIRMWARE_XOR, PAYLOAD_XOR, xor_firmware, xor_payload};
pub use packet::describe;

pub mod commands;

pub use commands::SESSION_ID;
