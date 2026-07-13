# Development

## Project layout

```
.
├── Cargo.toml                 # workspace manifest
├── Makefile                   # build / cross-build / package helpers
├── crates/
│   ├── rustsheng-core/        # core library (see architecture.md)
│   │   └── tests/fw/          # reference .raw fixtures for integration tests
│   └── rustsheng/             # CLI binary
├── docs/                      # this documentation (en/ + ru/)
└── .github/workflows/         # CI (ci.yml) and release (release.yml)
```

## Toolchain & conventions

- Rust ≥ 1.97, `edition = "2024"`.
- Code comments and commit messages are in **English**; commits follow
  Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`).
- Every source file carries an SPDX header:
  ```
  // SPDX-License-Identifier: GPL-3.0-or-later
  // Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
  // Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).
  ```

## Everyday commands

```bash
cargo build --workspace                          # build
cargo test --workspace                           # tests (hardware-free)
cargo fmt --all                                  # format
cargo fmt --all -- --check                       # format check (CI)
cargo clippy --workspace --all-targets -- -D warnings   # lint (CI)
cargo build -p rustsheng-core --no-default-features     # core without serial/flash-v5
```

All of the above must pass before a change is merged; CI enforces them.

## Testing approach

- **Pure layers** (`protocol`, `eeprom`, `firmware`, `flash`) are unit-tested
  directly with known byte vectors, round-trips, and — for AES — a NIST
  known-answer test.
- **`client`** is tested with `MockTransport` (scripted request/response), so no
  hardware is required.
- **Integration tests** (`crates/rustsheng-core/tests/fixtures.rs`) read the
  reference `.raw` images under `tests/fw/` and skip gracefully if absent.
- Flash packets are cross-validated against `K5TOOL` at the byte level (e.g.
  `rustsheng pack` output matches `K5TOOL -pack` exactly).

## Adding a new command / protocol

- A new radio operation is a method on `Client<T>` plus a `protocol::commands`
  builder and a CLI subcommand; test it with `MockTransport`.
- A new bootloader protocol is a `FlashProtocol` impl and a beacon-id branch in
  `Client::wait_for_beacon`.

## Code provenance & license

`rustsheng` is a derivative work of GPL-3.0 software and is itself licensed
**GPL-3.0-or-later**. Copyright is shared:

- Jacek Lipkowski (SQ5BPF) — `k5prog`.
- OneOfEleven — `k5prog-win`.
- qrp73 — `K5TOOL`.
- Maxim Krutovercev (RD2W) — this Rust port.

Specific borrowings: the payload XOR key, CRC, session id, EEPROM commands and the
`ORIGINAL_WRITES` table come from `k5prog`; the firmware XOR key, ADC/RSSI and the
encrypted-firmware detection come from `k5prog-win`; the V2/V5 flash packet
formats, AES key/IV table and `pack` logic come from `K5TOOL`. The extended
flash-size limit for oversized custom firmware (0x14000) is derived from K5TOOL's `data.Length > 0x10000` heuristic.
