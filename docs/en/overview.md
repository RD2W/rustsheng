# Overview

## What is rustsheng?

`rustsheng` is a Rust rewrite of the Quansheng UV-K5 EEPROM/flash programmer. It
talks to the radio over a serial port (a USB-to-UART programming cable) and lets
you back up and restore the radio's configuration (the EEPROM), read diagnostic
values, and flash firmware.

It combines the functionality of two reference tools and cross-checks against a
third:

- [`k5prog`](https://github.com/sq5bpf/k5prog) — Jacek Lipkowski's original C CLI
  (the canonical protocol reference).
- [`k5prog-win`](https://github.com/OneOfEleven/k5prog-win) — OneOfEleven's
  Windows GUI (added encrypted-firmware handling and ADC/RSSI reads).
- [`K5TOOL`](https://github.com/qrp73/K5TOOL) — qrp73's toolkit (the reference
  for the V2/V5 flash protocols and AES flashing).

## Features

- **Scan** available serial ports.
- **Read EEPROM** — full image (`0x2000`) or an arbitrary range (`--offset`/`--size`, up to `0x10000`).
- **Write EEPROM** — full image in three modes (`original`, `most`, `all`) or a partial write at an offset.
- **Read / write calibration** — the `0x1e00..0x2000` region.
- **Reset** — reboot the radio.
- **Read ADC** — battery ADC value (firmware-dependent).
- **Read RSSI** — RSSI / noise / glitch (firmware-dependent).
- **Bootloader info** — wait for the flash-mode beacon and print the bootloader version.
- **Flash firmware** — accepts raw and vendor-encrypted `.bin`; auto-detects the
  bootloader protocol (V2 unencrypted or V5 AES-CBC-128) from the beacon.
- **Offline firmware tools** — `pack` / `unpack` (vendor format ⇄ raw) and `parse`
  (decode a hex datagram) without a radio.
- **Sniffer** — passively decode datagrams on the wire.
- **Verbose logging** — `-v`/`-vv`/`-vvv` map to info/debug/trace (trace prints
  hex tx/rx datagrams).

## Project status

- **EEPROM read/write, calibration, reset, ADC/RSSI, sniffer, pack/unpack/parse:**
  implemented and, where possible, validated against real hardware. EEPROM full
  reads were confirmed **byte-for-byte identical** to both `k5prog` and `K5TOOL`
  on a real UV-K5.
- **Firmware flashing (V2 and V5):** implemented and cross-validated at the packet
  level against `K5TOOL` (byte-exact), but **not validated by actually flashing a
  radio**. V5/AES support targets bootloader 5.00.01 and has no hardware test.
  Treat flashing as experimental and dangerous.
- **ADC/RSSI** depend on firmware support. Stock Quansheng firmware responds;
  some custom firmwares do not (the command simply times out — this is not a bug).

## Supported hardware

Supports all known radio revisions — V1 (DP32G030), V2 (PY32F030), V3 and K1
(PY32F071) — each with appropriate flash-size and EEPROM-range limits. CPU
detection is automatic from the firmware image (for offline operations) and from
the bootloader version the radio reports (for live operations).  Live flashing
validates that the firmware targets the connected radio's CPU and refuses to
proceed on a mismatch.  Firmware flashing (V2 and V5/AES) is cross-validated at
the packet level but NOT tested by flashing a real radio — treat it as
experimental.

### Determining the radio revision

The tool determines the connected radio's CPU from the **bootloader version**
that the radio broadcasts in flash mode (command `bootloader-info`):

| Boot version | CPU | Radio revision |
|-------------|-----|----------------|
| `2.*` / `3.*` / `4.*` | DP32G030 | V1 (K5/K6) |
| `1.*` | PY32F030 | V2 |
| `5.*` | DP32G030 | R5+ |
| `7.*` | PY32F071 | V3 / K1 |

EEPROM read/write ranges and flash-size limits are validated against the detected
CPU.
