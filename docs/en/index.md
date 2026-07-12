# rustsheng — Documentation (English)

Full documentation for **rustsheng**, a cross-platform command-line tool for
reading/writing the EEPROM and flashing the firmware of Quansheng UV-K5 (and
compatible) radios over a serial port.

> ⚠️ **Use at your own risk.** Writing a bad EEPROM image or flashing firmware
> can misconfigure or permanently brick your radio. Firmware flashing is **not
> validated on real hardware** — see [Overview](overview.md#project-status).

## Table of contents

1. [Overview](overview.md) — purpose, features, project status.
2. [Installation & building](installation.md) — requirements, build, cross-compilation, cargo features.
3. [Usage](usage.md) — every subcommand, options, examples, safety gating.
4. [Architecture](architecture.md) — workspace, core layers, `FlashProtocol`, clean-architecture rationale.
5. [Protocol](protocol.md) — framing, CRC, obfuscation, commands, EEPROM, flash V2/V5 + AES.
6. [Development](development.md) — project layout, testing, style, contributing, code provenance.

Russian documentation: [`docs/ru`](../ru/index.md).
