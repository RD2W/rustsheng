// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Integration tests against real reference `.raw` fixtures.
//!
//! The fixtures live in `tests/fw/` and are committed to the repository, so the
//! tests are self-contained and run everywhere (locally and in CI).

use std::path::PathBuf;

use rustsheng_core::eeprom;
use rustsheng_core::firmware::FirmwareImage;

/// Resolves a fixture path under `crates/rustsheng-core/tests/fw/`.
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fw")
        .join(name)
}

#[test]
fn reference_eeprom_dump_has_expected_size() {
    let bytes = std::fs::read(fixture("uvk5_reference_eeprom.raw"))
        .expect("reference EEPROM dump should be present in tests/fw");
    assert_eq!(bytes.len(), eeprom::SIZE);
}

#[test]
fn reference_flash_image_loads_as_raw() {
    let bytes = std::fs::read(fixture("uvk5_reference_firmware.raw"))
        .expect("reference firmware image should be present in tests/fw");
    let image = FirmwareImage::load(&bytes).expect("reference firmware should load");
    assert!(!image.data.is_empty());
    assert!(image.data.len() <= rustsheng_core::firmware::MAX_FLASH);
    assert!(image.embedded_version.is_none());
}
