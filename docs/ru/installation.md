# Установка и сборка

## Требования

- **Rust ≥ 1.97** (workspace использует `edition = "2024"`). Установка через
  [rustup](https://rustup.rs/).
- Последовательный порт / USB-UART кабель для программирования (например,
  CH340-кабель для UV-K5).
- **Только Linux:** для сборки зависимости `serialport` и перечисления портов
  требуются заголовочные файлы `libudev`:
  ```bash
  sudo apt-get install -y libudev-dev pkg-config
  ```

## Сборка

```bash
# Отладочная сборка всего workspace
cargo build --workspace

# Оптимизированная release-сборка (рекомендуется)
cargo build --release --workspace
# или:
make build
```

Исполняемый файл CLI создаётся по пути `target/release/rustsheng`.

## Фичи Cargo (core library)

Крейт `rustsheng-core` имеет две фичи, обе **включены по умолчанию**:

| Фича | Включает | Дополнительные зависимости |
|---------|---------|--------------------|
| `serial` | реальный `SerialTransport` (ввод/вывод через serial port) | `serialport` |
| `flash-v5` | прошивка AES-CBC-128 (бутлоадер V5) | `aes`, `cbc` |

Сборка ядра без них (например, для переиспользования чистого протокольного слоя
в другом проекте):

```bash
cargo build -p rustsheng-core --no-default-features
```

Без `flash-v5` запрос прошивки V5 возвращает `FlashError::V5Unavailable`.

## Запуск тестов

```bash
cargo test --workspace
# или:
make test
```

Набор тестов не требует оборудования: протокольный и доменный слои чистые, а
клиент тестируется через `MockTransport` в памяти. Интеграционные тесты читают
эталонные `.raw`-фикстуры, хранящиеся в
`crates/rustsheng-core/tests/fw/`.

## Кросс-компиляция

`Makefile` оборачивает `cargo` для кросс-сборки:

```bash
make install-targets                    # rustup target add ...
make build-all                          # все цели для текущей платформы
make build-platform PLATFORM=windows    # одна платформа
```

CI (`.github/workflows/release.yml`) собирает Linux (x86_64, aarch64 через
`cross`), Windows (x86_64 через mingw-w64) и macOS (x86_64, aarch64) при пуше
тегов.

## Упаковка

`make package` собирает release-бинарник и архивирует его с версией из
`Cargo.toml`:

```bash
make version    # выводит, например, 0.1.0
make package    # сборки/rustsheng_v0.1.0_<платформа>.tar.gz (или .zip на Windows)
```
