# Архитектура

## Workspace

`rustsheng` — это Cargo workspace из двух крейтов:

```
crates/
├── rustsheng-core/   # core library — без CLI/UI фреймворков
└── rustsheng/        # CLI-бинарник на основе clap
```

Разделение следует принципам чистой архитектуры: ядро — переиспользуемая,
аппаратно-независимая, полностью тестируемая библиотека; CLI — один из
фронтендов. Позже можно добавить GUI как ещё один член workspace,
переиспользующий `rustsheng-core`.

## Направление зависимостей

Зависимости направлены **внутрь** (ядро не зависит от CLI):

```
rustsheng (CLI)
   └─> client ─> protocol ─> transport (trait)
        │                         ▲
        └─> eeprom / firmware / flash    SerialTransport (impl serialport, feature "serial")
```

- `protocol` и доменные модули (`eeprom`, `firmware`, `flash`) — **чистые**
  (без ввода/вывода) и тестируются напрямую модульными тестами.
- `client` зависит от абстракции `Transport`, а не от конкретного порта,
  поэтому тестируется с `MockTransport` в памяти.
- Ядро никогда не выполняет ввод/вывод напрямую помимо трейта `Transport`,
  никогда не вызывает `process::exit` и никогда не паникует на входных данных
  от радиостанции — возвращает типизированные `Result`.

## Модули ядра

```
rustsheng-core/src/
├── lib.rs
├── protocol/
│   ├── crc.rs           # CRC-16/XMODEM
│   ├── obfuscation.rs   # 16-байтный XOR-ключ для payload, 128-байтный XOR-ключ для прошивки
│   ├── frame.rs         # frame()/deframe(), FrameScanner, ProtocolError
│   ├── commands.rs      # построители командных payload + SESSION_ID
│   └── packet.rs        # describe() — человекочитаемое имя пакета
├── transport/
│   ├── mod.rs           # трейт Transport, TransportError, PortInfo
│   ├── serial.rs        # SerialTransport (feature "serial") + scan_ports
│   └── mock.rs          # MockTransport (тесты)
├── eeprom.rs            # размеры, режимы записи, итераторы блоков, калибровка
├── firmware.rs          # FirmwareImage: detect/decrypt/version-strip; pack()
├── flash/
│   ├── mod.rs           # трейт FlashProtocol, FlashKind, FlashError, build_sequence, blocks
│   ├── v2.rs            # ProtocolV2 (без шифрования)
│   └── v5.rs            # ProtocolV5 (AES-CBC-128, feature "flash-v5")
└── client.rs            # Client<T: Transport> — высокоуровневые операции
```

### transport
`Transport` — байтовая абстракция:
```rust
pub trait Transport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn read_exact_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
    fn flush_input(&mut self) -> Result<(), TransportError>;
    fn read_available(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
}
```
`SerialTransport` (фича `serial`) оборачивает крейт `serialport` (8N1, по
умолчанию 38400 бод). `MockTransport` воспроизводит пары запрос/ответ для
тестов.

### protocol
Чистая логика проводного протокола: CRC-16/XMODEM, XOR (де)обфускация,
кадрирование датаграмм (`frame`/`deframe`), потоковый `FrameScanner`
(используется сниффером), типизированные построители команд и `describe()` для
именования пакетов. См. [Протокол](protocol.md).

### client
`Client<T: Transport>` реализует сценарии использования: `connect`/`hello`,
`read_eeprom`/`read_region`/`write_block`, `reset`, `read_adc`/`read_rssi`,
`wait_for_beacon` и `flash_firmware`. Он кадрирует команду, записывает её,
читает ответ, дефреймит его и проверяет ответ — всё через внедрённый транспорт.

### flash
Трейт `FlashProtocol` абстрагирует протокол прошивки бутлоадера; `ProtocolV2`
и `ProtocolV5` его реализуют. `build_sequence` — чистый построитель, создающий
полный кадрированный поток пакетов для образа (используется в `--dry-run` и
тестах); `Client::flash_firmware` выполняет интерактивный поток beacon → версия
→ запись/подтверждение блоками, выбирая реализацию по идентификатору beacon.
См. [Протокол](protocol.md#firmware-flashing).

## Обработка ошибок

- Ядро: `thiserror`-перечисления на каждый слой — `TransportError`,
  `ProtocolError`, `ClientError`, `EepromError`, `FirmwareError`, `FlashError`.
- CLI: `anyhow` с `.context(...)` для человекочитаемых сообщений и корректных
  кодов выхода.

## Логирование

Ядро генерирует `log`-записи — `trace!` (hex tx/rx), `debug!` (на уровне
операций), `info!` (вехи). CLI инициализирует `env_logger` и сопоставляет
повторения `-v` с уровнями (warn → info → debug → trace).
