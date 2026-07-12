//! Интеграционные тесты на эталонных `.raw` файлах из репозитория.

use std::path::PathBuf;

use rustsheng_core::eeprom;
use rustsheng_core::firmware::FirmwareImage;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../k5prog_to_porting/k5prog")
        .join(name)
}

#[test]
fn reference_eeprom_dump_has_expected_size() {
    let bytes = std::fs::read(fixture("uvk5_original_eeprom.raw")).unwrap();
    assert_eq!(bytes.len(), eeprom::SIZE);
}

#[test]
fn reference_flash_image_loads_as_raw() {
    let bytes = std::fs::read(fixture("k5_flash.raw")).unwrap();
    let image = FirmwareImage::load(&bytes).expect("k5_flash.raw должен загружаться");
    assert!(!image.data.is_empty());
    assert!(image.data.len() <= rustsheng_core::firmware::MAX_FLASH);
    assert!(image.embedded_version.is_none());
}
