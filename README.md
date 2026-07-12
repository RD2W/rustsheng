# rustsheng

<!-- Update the OWNER (RD2W) below to match the published GitHub repository if different. -->
[![CI](https://github.com/RD2W/rustsheng/actions/workflows/ci.yml/badge.svg)](https://github.com/RD2W/rustsheng/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/RD2W/rustsheng?display_name=tag&sort=semver)](https://github.com/RD2W/rustsheng/releases)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.97%2B-orange.svg)](https://www.rust-lang.org)
![Edition](https://img.shields.io/badge/edition-2024-orange.svg)
![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey.svg)

A cross-platform command-line tool for reading/writing the EEPROM and flashing the
firmware of **Quansheng UV-K5** (and compatible) radios over a serial port —
a clean-architecture Rust rewrite of `k5prog` / `k5prog-win`, cross-checked
against `K5TOOL`.

**Language / Язык:** [English](#english) · [Русский](#русский)

> ⚠️ **Disclaimer / Use at your own risk.** Writing a bad EEPROM image or flashing
> firmware can misconfigure or **permanently brick** your radio. Firmware
> flashing (V2 and V5/AES) is **cross-validated at the packet level but NOT tested
> by actually flashing a radio** — treat it as experimental. You assume all risk.

---

## English

### Features

- Scan serial ports; read/write **EEPROM** (full image or arbitrary range).
- Three write modes (`original`/`most`/`all`) and partial writes; **calibration** read/write.
- **Reset**, battery **ADC**, **RSSI**/noise/glitch (firmware-dependent).
- **Flash firmware** (raw or vendor-encrypted `.bin`) with beacon-selected protocol
  (V2 unencrypted, V5 AES-CBC-128).
- Offline tools: `pack`/`unpack` firmware, `parse` a datagram, `sniffer`.
- Verbose logging (`-v`/`-vv`/`-vvv`).

### Quick start

```bash
# Build (needs Rust >= 1.97; on Linux: sudo apt-get install libudev-dev pkg-config)
cargo build --release --workspace       # or: make build

BIN=./target/release/rustsheng
$BIN scan-ports                                   # find the port
$BIN read-eeprom -p /dev/ttyUSB0 -o backup.raw    # back up EEPROM (radio in normal mode)
$BIN write-eeprom -p /dev/ttyUSB0 -i backup.raw --mode original
```

### Documentation

Full English documentation lives in [`docs/en`](docs/en/index.md):

- [Overview](docs/en/overview.md) — purpose, features, project status
- [Installation & building](docs/en/installation.md) — requirements, features, cross-compilation, tests
- [Usage](docs/en/usage.md) — every command, options, examples, safety gating
- [Architecture](docs/en/architecture.md) — workspace, core layers, clean architecture
- [Protocol](docs/en/protocol.md) — framing, CRC, obfuscation, commands, flash V2/V5 + AES
- [Development](docs/en/development.md) — layout, testing, contributing, code provenance

### Acknowledgements

This project stands on the work of others and is deeply grateful to them:

- **Jacek Lipkowski (SQ5BPF)** — [`k5prog`](https://github.com/sq5bpf/k5prog), the
  original UV-K5 EEPROM/flash programmer and protocol reverse-engineering.
- **OneOfEleven** — [`k5prog-win`](https://github.com/OneOfEleven/k5prog-win), the
  Windows GUI that added encrypted-firmware handling and ADC/RSSI reads.
- **qrp73** — [`K5TOOL`](https://github.com/qrp73/K5TOOL), the reference for the
  V2/V5 flash protocols, AES flashing, and firmware pack/unpack.
- **DualTachyon** and contributors —
  [`uv-k5-firmware`](https://github.com/DualTachyon/uv-k5-firmware), an open
  firmware that documents the radio side of the protocol.

Thank you for your work. 73 de RD2W!

### License

**GPL-3.0-or-later** (a derivative work of the GPL-3.0 tools above). See
[LICENSE](LICENSE). Copyright © 2023 Jacek Lipkowski SQ5BPF, © 2023 OneOfEleven,
© 2024 qrp73, © 2026 Maxim Krutovercev (RD2W).

---

## Русский

`rustsheng` — кроссплатформенный CLI-инструмент для чтения/записи EEPROM и
прошивки firmware радиостанций **Quansheng UV-K5** через последовательный порт.
Переписан на Rust по принципам чистой архитектуры на основе `k5prog` /
`k5prog-win` и сверен с `K5TOOL`.

> ⚠️ **Используйте на свой риск.** Неверный образ EEPROM или прошивка могут
> сбить настройки или **безвозвратно превратить радио в «кирпич»**. Прошивка
> (V2 и V5/AES) **сверена на уровне пакетов, но НЕ проверялась реальной
> записью в радио** — считайте её экспериментальной.

### Возможности

- Скан портов; чтение/запись **EEPROM** (полный образ или произвольный диапазон).
- Три режима записи (`original`/`most`/`all`) и частичная запись; чтение/запись **калибровки**.
- **Сброс**, **ADC** батареи, **RSSI**/шум/glitch (зависит от прошивки).
- **Прошивка** (raw или зашифрованный вендорский `.bin`) с выбором протокола по
  beacon (V2 без шифрования, V5 AES-CBC-128).
- Офлайн-инструменты: `pack`/`unpack` прошивки, `parse` датаграммы, `sniffer`.
- Подробное логирование (`-v`/`-vv`/`-vvv`).

### Быстрый старт

```bash
# Сборка (нужен Rust >= 1.97; на Linux: sudo apt-get install libudev-dev pkg-config)
cargo build --release --workspace       # или: make build

BIN=./target/release/rustsheng
$BIN scan-ports                                   # найти порт
$BIN read-eeprom -p /dev/ttyUSB0 -o backup.raw    # бэкап EEPROM (радио в обычном режиме)
$BIN write-eeprom -p /dev/ttyUSB0 -i backup.raw --mode original
```

### Документация

Полная русскоязычная документация — в [`docs/ru`](docs/ru/index.md):

- [Обзор](docs/ru/overview.md) — назначение, возможности, статус проекта
- [Установка и сборка](docs/ru/installation.md) — требования, фичи, кросс-компиляция, тесты
- [Использование](docs/ru/usage.md) — все команды, опции, примеры, safety-гейтинг
- [Архитектура](docs/ru/architecture.md) — workspace, слои ядра, чистая архитектура
- [Протокол](docs/ru/protocol.md) — кадры, CRC, обфускация, команды, флеш V2/V5 + AES
- [Разработка](docs/ru/development.md) — раскладка, тесты, вклад, происхождение кода

### Благодарности

Проект опирается на труд других людей, и мы искренне благодарны им:

- **Jacek Lipkowski (SQ5BPF)** — [`k5prog`](https://github.com/sq5bpf/k5prog),
  оригинальный программатор UV-K5 и реверс-инжиниринг протокола.
- **OneOfEleven** — [`k5prog-win`](https://github.com/OneOfEleven/k5prog-win),
  Windows-GUI с расшифровкой прошивок и чтением ADC/RSSI.
- **qrp73** — [`K5TOOL`](https://github.com/qrp73/K5TOOL), эталон протоколов
  прошивки V2/V5, AES и pack/unpack.
- **DualTachyon** и контрибьюторы —
  [`uv-k5-firmware`](https://github.com/DualTachyon/uv-k5-firmware), открытая
  прошивка, документирующая протокол со стороны радио.

Спасибо за ваш труд! 73 de RD2W!

### Лицензия

**GPL-3.0-or-later** (производная работа от GPL-3.0 инструментов выше). См.
[LICENSE](LICENSE). © 2023 Jacek Lipkowski SQ5BPF, © 2023 OneOfEleven,
© 2024 qrp73, © 2026 Максим Крутоверцев (RD2W).
