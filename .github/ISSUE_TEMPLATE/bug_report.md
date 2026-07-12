---
name: Bug report
about: Report a problem with rustsheng
title: "[bug] "
labels: bug
---

## Description

A clear description of the bug.

## Steps to reproduce

1. Command run (exact, including flags): `rustsheng ...`
2. ...

## Expected vs actual

- **Expected:**
- **Actual:**

## Verbose output

Re-run with `-vvv` and paste the relevant output (it includes hex tx/rx
datagrams). Redact anything sensitive.

```
<paste -vvv output here>
```

## Environment

- rustsheng version: `rustsheng --version`
- OS / architecture:
- Rust version: `rustc --version`
- Radio model + firmware version (from `read-eeprom`/`bootloader-info` if known):
- Serial cable / chip (e.g. CH340):

## Notes

- [ ] This is **not** a flashing (`flash`) problem I created by ignoring the
      "not hardware-validated" warning. (If it is, please say so explicitly.)
