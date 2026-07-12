# Usage

```
rustsheng [OPTIONS] <COMMAND>
```

Run `rustsheng --help` or `rustsheng <command> --help` for the built-in reference.

## Global options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Increase verbosity (repeatable): `-v` = info, `-vv` = debug, `-vvv` = trace (hex tx/rx). |
| `-h, --help` | Print help. |
| `-V, --version` | Print name, version and author. |

Radio subcommands take a connection:

| Option | Description |
|--------|-------------|
| `-p, --port <PORT>` | Serial port, e.g. `/dev/ttyUSB0` or `COM3`. |
| `-s, --speed <BAUD>` | Serial speed; default **38400** (the UV-K5's rate). |

Enter the radio's **normal mode** (power on *without* PTT) for EEPROM/calibration/
ADC/RSSI/reset. Enter **flash mode** (power on *while holding PTT*) for
`bootloader-info` and `flash`.

## Commands

### `scan-ports`
Lists serial ports the OS exposes. No radio interaction.
```bash
rustsheng scan-ports
```

### `read-eeprom` (alias `r`)
Reads the EEPROM to a file. Full image by default, or a range with `--offset`/`--size` (decimal or `0x` hex; addressable range up to `0x10000`).
```bash
rustsheng read-eeprom -p /dev/ttyUSB0 -o backup.raw
rustsheng read-eeprom -p /dev/ttyUSB0 --offset 0x1e00 --size 0x200 -o calib.raw
```

### `write-eeprom` (alias `w`)
Writes the EEPROM from a file. Full-image mode requires exactly `0x2000` bytes and takes `--mode`:

| Mode | Writes |
|------|--------|
| `original` (default) | the vendor block set/order (safest) |
| `most` | everything up to `0x1d00` (no calibration) — needs no confirmation |
| `all` | the entire EEPROM incl. calibration — needs `--i-know-what-im-doing` |

Partial write places a file at an arbitrary `--offset` (advanced; needs `--i-know-what-im-doing`):
```bash
rustsheng write-eeprom -p /dev/ttyUSB0 -i backup.raw --mode original
rustsheng write-eeprom -p /dev/ttyUSB0 -i patch.bin --offset 0x0e70 --i-know-what-im-doing
```

### `read-calibration` / `write-calibration`
Read/write the calibration region `0x1e00..0x2000` (`0x200` bytes). Writing calibration needs `--i-know-what-im-doing`.
```bash
rustsheng read-calibration -p /dev/ttyUSB0 -o calib.raw
```

### `reset`
Reboots the radio.

### `read-adc` / `read-rssi`
Read the battery ADC, or RSSI/noise/glitch. Firmware-dependent (may time out on custom firmware).

### `bootloader-info`
Waits for the flash-mode beacon and prints the bootloader version and detected protocol (V2/V5). Requires the radio in flash mode.

### `flash` (alias `F`) — **dangerous, not hardware-validated**
Flashes a firmware image (raw or vendor-encrypted `.bin`, auto-detected). Accepts
firmware images for V1, V2, V3 and K1 radios (including custom builds). The
bootloader protocol (V2 or V5) is chosen from the beacon.

| Option | Description |
|--------|-------------|
| `-i, --input <FILE>` | Firmware image. |
| `-M, --fw-version <VER>` | Version string sent to the bootloader (default `*.01.23`). |
| `--key-number <0..15>` | V5 AES key pair (default 0). |
| `--dry-run` | Build the packet stream and write it to a file **without opening the port**. |
| `--protocol v2\|v5` | Protocol for `--dry-run` (live mode auto-detects). |
| `-o, --output <FILE>` | `--dry-run` output (default `<input>.packets.bin`). |
| `--i-know-what-im-doing` | Repeatable confirmation. Live V2 needs ≥3, V5 needs ≥5. |

```bash
# Safe offline packet preview (no radio, no confirmation):
rustsheng flash -i firmware.bin --dry-run -o packets.bin

# Live flash (radio in flash mode) — REQUIRES confirmation:
rustsheng flash -p /dev/ttyUSB0 -i firmware.bin \
    --i-know-what-im-doing --i-know-what-im-doing --i-know-what-im-doing
```

### `unpack` / `pack` — offline firmware tools
`unpack` decrypts a vendor-packed image to raw; `pack` builds a vendor image
(inserts the 16-byte version at `0x2000`, XOR-obfuscates, appends CRC). No radio.
```bash
rustsheng unpack -i vendor.bin -o firmware.raw
rustsheng pack   -i firmware.raw -M "2.01.23" -o vendor.bin
```

### `parse` — offline datagram decoder
Decodes a hex datagram and names the packet.
```bash
rustsheng parse "ab cd 08 00 02 69 10 e6 44 a8 5a 24 b9 a9 dc ba"
# Payload (8 bytes): 14 05 04 00 6a 39 57 64
# Packet: HelloReq (0x0514)
```

### `sniffer`
Passively reads the port, extracts datagrams from the stream and prints their decoded form until Ctrl-C. Useful in flash mode, where the radio broadcasts a beacon.
```bash
rustsheng sniffer -p /dev/ttyUSB0
```

## Safety gating

Dangerous operations refuse to run without a repeatable `--i-know-what-im-doing`
(mirroring `-Y` in the original `k5prog`):

| Operation | Required level |
|-----------|----------------|
| `write-eeprom --mode all` | ≥ 1 |
| `write-eeprom --offset` (partial) | ≥ 1 |
| `write-calibration` | ≥ 1 |
| `flash` (V2) | ≥ 3 |
| `flash` (V5) | ≥ 5 |
| `flash` of a suspiciously small image | ≥ level + 2 |

## Troubleshooting

- **No `/dev/ttyUSB*`:** check the cable/driver; `scan-ports` lists what the OS sees.
- **Permission denied on Linux:** add yourself to the `dialout` group
  (`sudo usermod -aG dialout $USER`, then re-login).
- **`read-adc`/`read-rssi` time out:** the firmware doesn't implement those
  serial commands (common on custom firmware).
- **Nothing decodes / bad CRC:** try `-vvv` to see raw tx/rx; confirm the port and 38400 baud.
