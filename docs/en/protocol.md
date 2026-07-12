# Protocol

The programming protocol was reverse-engineered by the authors of the reference
tools; this document summarizes what `rustsheng` implements. Byte layouts are
taken verbatim from `k5prog`, `k5prog-win` and `K5TOOL`.

## Datagram framing

Every message is framed as:

```
AB CD | len_lo len_hi | <payload> | crc_lo crc_hi | DC BA
```

- `len` is the payload length (little-endian, 16-bit).
- Only the **payload and its 2 CRC bytes** are obfuscated (XOR); the header,
  length and footer are sent in the clear.
- The CRC is **CRC-16/XMODEM** (poly `0x1021`, init `0x0000`) over the clear
  payload.

### Obfuscation
XOR with a repeating 16-byte key:
```
16 6c 14 e6 2e 91 0d 40 21 35 d5 40 13 03 e9 80
```
(Firmware images use a separate 128-byte XOR key — see [firmware](#firmware-images).)

### CRC on received datagrams
The radio treats the CRC as obfuscation, not integrity: replies carry `0xFFFF`
either already obfuscated (EEPROM replies) or in the clear (the flash-mode
beacon). `deframe` therefore accepts a datagram when **either** the raw
(on-the-wire) CRC field **or** the de-obfuscated CRC is `0xFFFF`, or when a
genuine CRC matches — matching the reference tools.

## Session id

Most commands echo a 4-byte session id / timestamp, constant within a session:
```
6a 39 57 64
```

## Commands (normal mode)

Addresses are little-endian. `id` = first two payload bytes (little-endian).

| Command | id | Payload |
|---------|----|---------|
| Hello | `0x0514` | `14 05 04 00 <session>` |
| Hello ack | `0x0515` | version (16 bytes) at offset 4 |
| Read EEPROM | `0x051b` | `1b 05 08 00 addr_lo addr_hi len 00 <session>` (len ≤ 0x80) |
| Read ack | `0x051c` | data from byte 8 |
| Write EEPROM | `0x051d` | `1d 05 <8+len> 00 addr_lo addr_hi len 01 <session> <data>` |
| Write ack | `0x051e` | echoes address |
| Read RSSI | `0x0527` | `27 05 04 00 <session>` |
| RSSI ack | `0x0528` | rssi (9-bit) `[4..6] & 0x1ff`, noise `[6] & 0x7f`, glitch `[7]` |
| Read ADC | `0x0529` | `29 05 04 00 <session>` |
| ADC ack | `0x052a` | voltage `[4..6]`, current `[6..8]` |
| Reset | `0x05dd` | `dd 05 00 00` |

RSSI is reported as `raw/2 − 160` dBm. ADC→volts needs the radio's per-unit
calibration (`0x1f40`): `V = 7.6 × adc / calib[3]`.

## EEPROM geometry

| Constant | Value |
|----------|-------|
| Size | `0x2000` |
| Block | `0x80` |
| Size without calibration | `0x1d00` |
| Calibration region | `0x1e00..0x2000` (`0x200`) |
| Addressable range (range ops) | `0x10000` |

Write modes:
- **Original** — the vendor block list/order (from `uvk5.h`).
- **Most** — `0x0000..0x1d00`.
- **All** — the whole `0x2000` (includes calibration).

## Firmware images

`FirmwareImage::load` accepts both formats:

- **Raw** — recognized by the ARM vector-table signature
  (`data[2]=0x00, data[3]=0x20, data[6]=0x00, data[10]=0x00, data[14]=0x00`);
  used as-is.
- **Vendor-packed** — CRC-terminated and XOR-obfuscated with the 128-byte
  firmware key. Verified by CRC, decrypted, then the 16-byte version string at
  `0x2000` is extracted and stripped.

`pack()` is the inverse: insert the version at `0x2000`, XOR-obfuscate, append the
little-endian CRC. `rustsheng pack` produces output **byte-identical** to
`K5TOOL -pack`. Flash size limit: `0xf000` (the bootloader lives above it).

Firmware images larger than the classic V1 limit (0xf000) are automatically
allowed up to 0x14000 to support V2/V3/K1 and custom firmware. Loading enforces
the two-tier limit; the flash path adds a CPU-level cap.

## Firmware flashing

Flashing is beacon-driven. The bootloader broadcasts a beacon; its id selects the
protocol:

| | Beacon | Version req | Write req | Write ack |
|--|--------|-------------|-----------|-----------|
| **V2** (unencrypted) | `0x0518` | `0x0530` | `0x0519` | `0x051a` |
| **V5** (AES-CBC-128) | `0x057a` | `0x057d` | `0x057b` | `0x057c` |

Write request layout (both, 272-byte payload; V5 encrypts the data):
```
<cmd> 05 0c 01 | 8a 8d 9f 1d | chunk_no(LE16) chunk_count(LE16) len(LE16) 00 00 | <0x100 data>
```
- `cmd` = `0x19` (V2) or `0x7b` (V5); `id` = `0x1d9f8d8a` → bytes `8a 8d 9f 1d`.
- `chunk_no = offset / 0x100`; data is a full `0x100` page, unused tail filled `0xff`.
- Write ack: `chunk_no` at `[8..10]`, result at `[10]` (0 = OK).

### V5 encryption
V5 encrypts each `0x100` page with **AES-128-CBC (no padding)**, chained across
the whole image (one keystream per session). The version request (`0x057d`)
selects one of 16 key/IV pairs by `key_number`; each key and IV is reversed
per 32-bit word before use. The AES implementation is validated with a NIST
SP800-38A known-answer vector.

Flash flow: `wait_for_beacon` → pick protocol → send version request → per page:
build the write request, send, await the ack (ignoring repeated beacons, up to
5 attempts), verify `chunk_no` and result.
