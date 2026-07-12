# Разработка

## Структура проекта

```
.
├── Cargo.toml                 # манифест workspace
├── Makefile                   # сборка / кросс-сборка / упаковка
├── crates/
│   ├── rustsheng-core/        # core library (см. architecture.md)
│   │   └── tests/fw/          # эталонные .raw-фикстуры для интеграционных тестов
│   └── rustsheng/             # CLI-бинарник
├── docs/                      # эта документация (en/ + ru/)
└── .github/workflows/         # CI (ci.yml) и релиз (release.yml)
```

## Инструментарий и соглашения

- Rust ≥ 1.97, `edition = "2024"`.
- Комментарии в коде и сообщения коммитов — на **английском**; коммиты следуют
  Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`).
- Каждый исходный файл содержит SPDX-заголовок:
  ```
  // SPDX-License-Identifier: GPL-3.0-or-later
  // Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
  // Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).
  ```

## Повседневные команды

```bash
cargo build --workspace                          # сборка
cargo test --workspace                           # тесты (без оборудования)
cargo fmt --all                                  # форматирование
cargo fmt --all -- --check                       # проверка форматирования (CI)
cargo clippy --workspace --all-targets -- -D warnings   # линтинг (CI)
cargo build -p rustsheng-core --no-default-features     # ядро без serial/flash-v5
```

Все перечисленные команды должны проходить перед слиянием изменений; CI
обеспечивает их соблюдение.

## Подход к тестированию

- **Чистые слои** (`protocol`, `eeprom`, `firmware`, `flash`) тестируются
  модульными тестами напрямую — с известными байтовыми векторами,
  round-trip-тестами и, для AES, тестом с известным ответом NIST.
- **`client`** тестируется с `MockTransport` (заскриптованные запрос/ответ),
  поэтому оборудование не требуется.
- **Интеграционные тесты** (`crates/rustsheng-core/tests/fixtures.rs`) читают
  эталонные `.raw`-образы из `tests/fw/` и корректно пропускаются при их
  отсутствии.
- Пакеты flash сверены с `K5TOOL` на байтовом уровне (например, вывод
  `rustsheng pack` побайтово совпадает с `K5TOOL -pack`).

## Добавление новой команды / протокола

- Новая операция радиостанции — метод на `Client<T>` плюс построитель в
  `protocol::commands` и подкоманда CLI; тестируется с `MockTransport`.
- Новый протокол бутлоадера — реализация `FlashProtocol` и ветка по
  идентификатору beacon в `Client::wait_for_beacon`.

## Происхождение кода и лицензия

`rustsheng` является производной работой от программ под GPL-3.0 и сам
лицензирован под **GPL-3.0-or-later**. Авторские права разделены:

- Jacek Lipkowski (SQ5BPF) — `k5prog`.
- OneOfEleven — `k5prog-win`.
- qrp73 — `K5TOOL`.
- Maxim Krutovercev (RD2W) — данный Rust-порт.

Конкретные заимствования: XOR-ключ для payload, CRC, идентификатор сессии,
команды EEPROM и таблица `ORIGINAL_WRITES` взяты из `k5prog`; XOR-ключ для
прошивки, ADC/RSSI и определение зашифрованной прошивки — из `k5prog-win`;
форматы пакетов V2/V5 flash, таблица ключей/IV AES и логика `pack` — из
`K5TOOL`.
