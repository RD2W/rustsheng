# Security Policy

## Scope

`rustsheng` talks to radio hardware over a local serial port. The most serious
risks are **operational** rather than remote:

- **Bricking a radio** by flashing a bad image. Firmware flashing (V2 and V5/AES)
  is cross-validated at the packet level but **not validated by flashing real
  hardware** — it is gated behind repeated `--i-know-what-im-doing` confirmations
  for this reason.
- Writing a bad EEPROM/calibration image that misconfigures the radio.

There is no network attack surface: the tool only reads/writes a serial port and
local files.

## Reporting a vulnerability

Please report security issues **privately**, not via public issues:

- Use GitHub's "Report a vulnerability" (Security → Advisories), or
- email the maintainer: `mkrutovercev@yandex.ru`.

Include steps to reproduce and the impact. We'll acknowledge and work on a fix; a
coordinated disclosure timeline can be agreed if needed.

## Supported versions

This is pre-1.0 software; fixes land on the latest `dev`/release. Pin a released
tag for reproducible builds (`Cargo.lock` is committed).
