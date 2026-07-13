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
- Firmware loading now supports V2 (PY32F030), V3 and K1 (PY32F071) radio
  revisions, including custom firmware images. Images larger than the classic
  V1 limit (0xf000) are automatically allowed — per-CPU limits (0x10000 V2,
  0x12000 V3/K1) with an extended cap of 0x14000 for oversized custom images.
- Per-CPU flash-size limits and automatic CPU detection from the vector table.
- Bootloader-version to CPU mapping (`Cpu::from_boot_version`) — derived from
  forum reports of the TWHH (OUROBOROS) flasher tools.  Live flashing checks
  that the firmware image targets the connected radio's CPU and refuses to
  proceed on a mismatch.
- Per-CPU EEPROM read/write range limits (V1 → 0x2000, V2/V3/K1 → 0x10000),
  validated against the bootloader-reported CPU at runtime.
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
