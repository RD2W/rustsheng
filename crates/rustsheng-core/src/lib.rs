// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! `rustsheng-core`: hardware-independent core for programming Quansheng UV-K5
//! radios over serial. Contains the wire protocol, high-level operations, and a
//! [`transport::Transport`] abstraction so the logic can be exercised without
//! real hardware.

pub mod client;
pub use client::{Client, ClientError};
pub mod eeprom;
pub mod firmware;
pub mod protocol;
pub mod transport;
pub mod flash;
