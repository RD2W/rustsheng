# Architecture

## Workspace

`rustsheng` is a Cargo workspace with two crates:

```
crates/
├── rustsheng-core/   # core library — no CLI/UI frameworks
└── rustsheng/        # clap-based CLI binary
```

The split follows clean-architecture principles: the core is a reusable,
hardware-independent, fully testable library; the CLI is one frontend. A GUI
could be added later as another workspace member reusing `rustsheng-core`.

## Dependency direction

Dependencies point **inward** (nothing in the core depends on the CLI):

```
rustsheng (CLI)
   └─> client ─> protocol ─> transport (trait)
        │                         ▲
        └─> eeprom / firmware / flash    SerialTransport (impl serialport, feature "serial")
```

- `protocol` and the domain modules (`eeprom`, `firmware`, `flash`) are **pure**
  (no I/O) and unit-tested directly.
- `client` depends on the `Transport` **abstraction**, not a concrete port, so it
  is tested with an in-memory `MockTransport`.
- The core never performs I/O directly beyond the `Transport` trait, never calls
  `process::exit`, and never panics on radio input — it returns typed `Result`s.

## Core modules

```
rustsheng-core/src/
├── lib.rs
├── protocol/
│   ├── crc.rs           # CRC-16/XMODEM
│   ├── obfuscation.rs   # 16-byte payload XOR key, 128-byte firmware XOR key
│   ├── frame.rs         # frame()/deframe(), FrameScanner, ProtocolError
│   ├── commands.rs      # command payload builders + SESSION_ID
│   └── packet.rs        # describe() — human-readable packet naming
├── transport/
│   ├── mod.rs           # Transport trait, TransportError, PortInfo
│   ├── serial.rs        # SerialTransport (feature "serial") + scan_ports
│   └── mock.rs          # MockTransport (tests)
├── eeprom.rs            # sizes, write modes, block iterators, calibration
├── firmware.rs          # FirmwareImage: detect/decrypt/version-strip; pack()
├── flash/
│   ├── mod.rs           # FlashProtocol trait, FlashKind, FlashError, build_sequence, blocks
│   ├── v2.rs            # ProtocolV2 (unencrypted)
│   └── v5.rs            # ProtocolV5 (AES-CBC-128, feature "flash-v5")
└── client.rs            # Client<T: Transport> — high-level operations
```

### transport
`Transport` is the byte-level abstraction:
```rust
pub trait Transport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn read_exact_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
    fn flush_input(&mut self) -> Result<(), TransportError>;
    fn read_available(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
}
```
`SerialTransport` (feature `serial`) wraps the `serialport` crate (8N1, default
38400 baud). `MockTransport` scripts request/response pairs for tests.

### protocol
Pure wire-protocol logic: CRC-16/XMODEM, XOR (de)obfuscation, datagram framing
(`frame`/`deframe`), a streaming `FrameScanner` (used by the sniffer), typed
command builders, and `describe()` for packet naming. See [protocol.md](protocol.md).

### client
`Client<T: Transport>` implements the use cases: `connect`/`hello`,
`read_eeprom`/`read_region`/`write_block`, `reset`, `read_adc`/`read_rssi`,
`wait_for_beacon`, and `flash_firmware`. It frames a command, writes it, reads the
reply, deframes it, and validates the response — all over the injected transport.

### flash
The `FlashProtocol` trait abstracts the bootloader flash protocol; `ProtocolV2`
and `ProtocolV5` implement it. `build_sequence` is a pure builder that produces
the full framed packet stream for an image (used by `--dry-run` and tests);
`Client::flash_firmware` performs the interactive beacon → version → per-block
write/ack flow, choosing the impl from the beacon id. See [protocol.md](protocol.md#firmware-flashing).

## Error handling

- Core: `thiserror` enums per layer — `TransportError`, `ProtocolError`,
  `ClientError`, `EepromError`, `FirmwareError`, `FlashError`.
- CLI: `anyhow` with `.context(...)` for human-readable messages and correct
  exit codes.

## Logging

The core emits `log` records — `trace!` (hex tx/rx), `debug!` (per-operation),
`info!` (milestones). The CLI initializes `env_logger` and maps `-v` occurrences
to levels (warn → info → debug → trace).
