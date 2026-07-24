# rustsheng — скрипты проверки (read / write / flash)

Кроссплатформенные Python-скрипты для тестирования rustsheng на радио
Quansheng UV-K5 (V1/V2/V3/K1). Работают на Linux, macOS и Windows.

**Требования**: Python 3.12 или новее (только стандартная библиотека).

**Проще всего**: положите бинарник `rustsheng` (или `rustsheng.exe` на Windows)
в одну папку со скриптами — тогда `--bin` указывать не нужно, скрипты найдут
его автоматически. Если бинарник в `PATH`, тоже подхватится.

Бинарники для всех платформ: <https://github.com/RD2W/rustsheng/releases>

---

# 1. Read-only тест (чтение без записи)

Скрипт: **`rustsheng_test_read.py`**

Собирает данные с радио **БЕЗ** записи (только чтение).

## Что проверяется

Обычный режим (радио включено, PTT НЕ зажат):
- `scan-ports`        — список последовательных портов
- `read-adc`          — handshake + версия прошивки + ADC батареи
- `read-rssi`         — RSSI / шум / glitch
- `read-eeprom`       — полный дамп EEPROM (0x2000) в `eeprom.raw`
- `read-calibration`  — калибровочная область (0x200) в `calibration.raw`

Режим прошивки (опционально: выключить, зажать PTT + верхнюю боковую кнопку,
включить):
- `bootloader-info`   — определение протокола (V2/V5) и версии загрузчика
- `sniffer`           — 6 секунд пассивного прослушивания beacon'ов

## Запуск

```bash
chmod +x rustsheng_test_read.py

# Linux / macOS
./rustsheng_test_read.py --port /dev/ttyUSB0

# Windows (PowerShell или cmd)
python rustsheng_test_read.py --port COM3

# Явный путь к бинарнику:
./rustsheng_test_read.py -p /dev/ttyUSB0 --bin ./rustsheng
```

Если нет прав на порт (Linux): `sudo usermod -aG dialout $USER` и перелогиниться.
Номер COM-порта (Windows): Диспетчер устройств → «Порты (COM и LPT)».

## Что прислать разработчику

Папку `rustsheng-test-<дата>-<время>/` целиком:
- `report.txt`        — полный лог всех команд
- `eeprom.raw`        — дамп EEPROM (если чтение удалось)
- `calibration.raw`   — калибровка (если чтение удалось)

**ОБЯЗАТЕЛЬНО укажите ревизию радио (V1/V2/V3/K1) и версию прошивки.**

---

# 2. Write round-trip тест — только для тех, кто готов рискнуть

Скрипт: **`rustsheng_test_write.py`**

> **ВНИМАНИЕ: этот скрипт ПИШЕТ в EEPROM радио.**

Тестирует путь записи максимально безопасным способом:

1. читает полный дамп EEPROM в **бэкап** (`eeprom-backup.raw`);
2. записывает **тот же самый дамп** обратно (`write-eeprom --mode original`) — байты не меняются;
3. перечитывает EEPROM и **сверяет** с бэкапом.

Бэкап всегда сохраняется. Требует явного подтверждения (`yes`) перед записью.

## Запуск

```bash
chmod +x rustsheng_test_write.py

# Linux / macOS
./rustsheng_test_write.py --port /dev/ttyUSB0

# Windows
python rustsheng_test_write.py --port COM3

# Явный путь к бинарнику:
./rustsheng_test_write.py -p /dev/ttyUSB0 --bin ./rustsheng
```

## Восстановление

```
rustsheng write-eeprom -p <порт> -i eeprom-backup.raw --mode original
```

## Что прислать

Папку `rustsheng-write-test-<дата>/`:
- `report.txt`          — лог с результатом (УСПЕХ / РАСХОЖДЕНИЕ)
- `eeprom-backup.raw`   — исходный дамп
- `eeprom-verify.raw`   — дамп после записи

**Укажите ревизию радио (V1/V2/V3/K1) и версию прошивки.**

---

# 3. Прошивка (FLASH) — только для смелых

Скрипт: **`rustsheng_flash.py`**

> **ОПАСНО: прошивка может НЕОБРАТИМО превратить радио в кирпич.**

Скрипт делает всё максимально осторожно:

1. бэкап EEPROM и калибровки (обычный режим);
2. `flash --dry-run` — построить поток пакетов в файл, **ничего не пишет** в радио;
3. явное подтверждение `FLASH`;
4. реальная прошивка с нужным числом `--i-know-what-im-doing` (V2: 3, V5: 5).

## Запуск

```bash
chmod +x rustsheng_flash.py

# Linux / macOS / Windows — одинаково
./rustsheng_flash.py --port /dev/ttyUSB0 --firmware firmware.bin --protocol v2

# Для V3/K1 — принудительно указать CPU (обходит неверное автоопределение):
./rustsheng_flash.py -p COM4 -f firmware.bin --force-cpu py32f071

# Явный путь к бинарнику:
./rustsheng_flash.py -p /dev/ttyUSB0 -f firmware.bin --protocol v5 --bin ./rustsheng
```

## Важно
- **Протокол** в live-режиме rustsheng определяет сам по beacon; `--protocol` используется только для `--dry-run`.
- Прошивка отклоняется при несовпадении CPU образа и загрузчика радио. Для V3/K1 используйте `--force-cpu py32f071` — автоопределение ошибается на стоковых и кастомных образах PY32F071.
- Образ < 50 000 байт требует ещё +2 флага подтверждения.

## Восстановление EEPROM после прошивки

```
rustsheng write-eeprom -p <порт> -i eeprom-backup.raw --mode original
```

## Что прислать

Папку `rustsheng-flash-<дата>/` целиком:
- `report.txt`              — полный лог
- `eeprom-backup.raw`       — бэкап EEPROM (**ХРАНИТЕ!**)
- `calibration-backup.raw`  — бэкап калибровки
- `packets.bin`             — dry-run поток пакетов

**Укажите ревизию радио (V1/V2/V3/K1) и версию прошивки.**
