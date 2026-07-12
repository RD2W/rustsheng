// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Firmware flashing protocols (V2 unencrypted, V5 AES-CBC-128).

pub mod v2;
#[cfg(feature = "flash-v5")]
pub mod v5;
