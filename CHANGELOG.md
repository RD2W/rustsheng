# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Serial port scanning (`scan-ports`).
- EEPROM read/write: full image and arbitrary range (`--offset`/`--size`, up to
  `0x10000`); write modes `original`/`most`/`all`; partial write at an offset.
- Calibration read/write (`0x1e00..0x2000`).
- `reset`, `read-adc`, `read-rssi`, `bootloader-info`.
- Firmware flashing with beacon-selected protocol: **V2** (unencrypted) and
  **V5** (AES-CBC-128, behind the default `flash-v5` feature). Includes a
  `--dry-run` packet preview and V2≥3 / V5≥5 confirmation gating.
- Offline firmware tools: `pack`, `unpack`, `parse`.
- `sniffer` — passive datagram decoding.
- Verbose logging (`-v`/`-vv`/`-vvv`).
- Full bilingual documentation under `docs/en` and `docs/ru`.

### Notes
- Firmware flashing is cross-validated against `K5TOOL` at the packet level but
  **not tested by flashing real hardware**. Use at your own risk.
- EEPROM full reads were verified byte-for-byte against `k5prog` and `K5TOOL` on
  real hardware.

[Unreleased]: https://github.com/RD2W/rustsheng/commits/dev
