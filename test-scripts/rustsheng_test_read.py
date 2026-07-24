#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
#
# rustsheng: read-only test script (cross-platform Python).
# Собирает данные с радио БЕЗ записи (только чтение) для проверки на разных
# ревизиях UV-K5 (V1/V2/V3/K1) и разных прошивках.
#
# Использование:
#   ./rustsheng_test_read.py --port /dev/ttyUSB0
#   ./rustsheng_test_read.py --port COM3 --bin ./rustsheng.exe

import argparse
import datetime
import platform
import shutil
import subprocess
import sys
from pathlib import Path


class TeeLogger:
    """Пишет сообщения одновременно в stdout и в файл лога."""

    def __init__(self, log_path: Path) -> None:
        self.log_path = log_path
        self._file = log_path.open("w", encoding="utf-8")

    def write(self, msg: str) -> None:
        sys.stdout.write(msg)
        sys.stdout.flush()
        self._file.write(msg)
        self._file.flush()

    def close(self) -> None:
        self._file.close()


def find_binary(bin_name: str) -> str:
    resolved = shutil.which(bin_name)
    if resolved:
        return resolved
    path = Path(bin_name)
    if path.is_file():
        return str(path.resolve())
    script_dir = Path(sys.argv[0]).resolve().parent
    sibling = script_dir / bin_name
    if sibling.is_file():
        return str(sibling)
    return ""


def fmt_args(args: list[str]) -> str:
    return " ".join(args)


SUBPROCESS_TIMEOUT_SNIFFER = 8
SNIFFER_HEAD_LINES = 20


def main() -> None:
    if sys.platform == "win32":
        sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[union-attr]
        sys.stderr.reconfigure(encoding="utf-8")  # type: ignore[union-attr]

    default_bin = "rustsheng.exe" if sys.platform == "win32" else "rustsheng"

    parser = argparse.ArgumentParser(
        description="rustsheng read-only test (cross-platform)"
    )
    parser.add_argument(
        "--port", "-p", required=True, help="Последовательный порт (напр. /dev/ttyUSB0 или COM3)"
    )
    parser.add_argument(
        "--bin",
        default=default_bin,
        help=f"Путь к бинарнику rustsheng (по умолчанию: {default_bin})",
    )
    args = parser.parse_args()

    bin_path = find_binary(args.bin)
    if not bin_path:
        sys.exit(f"ОШИБКА: бинарник rustsheng не найден: {args.bin}")

    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    outdir = Path(f"rustsheng-test-{stamp}")
    outdir.mkdir(parents=True, exist_ok=True)
    log_path = outdir / "report.txt"

    tee = TeeLogger(log_path)

    def log(msg: str = "") -> None:
        tee.write(msg + "\n")

    def run(title: str, cmd_args: list[str], *, timeout: int | None = None) -> int:
        log("-------------------------------------------------")
        log(f"### {title}")
        log(f"$ {bin_path} {fmt_args(cmd_args)}")
        try:
            result = subprocess.run(
                [bin_path, *cmd_args],
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            log(result.stdout.rstrip())
            if result.stderr:
                log(result.stderr.rstrip())
            log(f"(код возврата: {result.returncode})")
            return result.returncode
        except subprocess.TimeoutExpired:
            log("(процесс остановлен по таймауту)")
            return -1
        except OSError as exc:
            log(f"(ошибка запуска: {exc})")
            return -1
        finally:
            log()

    os_info = platform.uname()
    log("==================================================")
    log(f" rustsheng read-only test — {stamp}")
    log(f" Порт:       {args.port}")
    log(f" Бинарник:   {bin_path}")
    log(f" ОС:         {os_info.system} {os_info.release} {os_info.version} {os_info.machine}")
    log(f" Python:     {sys.version}")
    log("==================================================")
    log()
    log(">>> ПОЖАЛУЙСТА, УКАЖИТЕ В ОТЧЁТЕ РЕВИЗИЮ РАДИО (V1/V2/V3/K1) И ПРОШИВКУ <<<")
    log()

    log("########## ОБЫЧНЫЙ РЕЖИМ ##########")
    log("Радио должно быть ВКЛЮЧЕНО в обычном режиме (PTT НЕ зажат).")
    log()

    run("Список портов", ["scan-ports"])
    run("Версия прошивки + ADC", ["read-adc", "-p", args.port])
    run("RSSI / шум / glitch", ["read-rssi", "-p", args.port])
    run(
        "Дамп EEPROM",
        ["read-eeprom", "-p", args.port, "-o", str(outdir / "eeprom.raw")],
    )
    run(
        "Калибровка (0x1e00)",
        ["read-calibration", "-p", args.port, "-o", str(outdir / "calibration.raw")],
    )

    log("########## РЕЖИМ ПРОШИВКИ (опционально) ##########")
    log(
        "Чтобы проверить: ВЫКЛЮЧИТЕ радио, зажмите PTT (+ верхнюю боковую кнопку)"
    )
    log("и включите — загорится подсветка, радио начнёт слать beacon.")
    log()
    log("Радио в режиме прошивки? Enter для теста или 's' чтобы пропустить:")
    ans = sys.stdin.readline().strip()
    log(f"> {ans}")
    if ans.lower() != "s":
        run("Bootloader info", ["bootloader-info", "-p", args.port])

        log("### Sniffer (6 секунд пассивного прослушивания)")
        log(f"$ {bin_path} sniffer -p {args.port}")
        try:
            result = subprocess.run(
                [bin_path, "sniffer", "-p", args.port],
                capture_output=True,
                text=True,
                timeout=SUBPROCESS_TIMEOUT_SNIFFER,
            )
            lines = (result.stdout + result.stderr).splitlines()[:SNIFFER_HEAD_LINES]
            log("\n".join(lines))
            log()
        except subprocess.TimeoutExpired:
            log("(процесс остановлен по таймауту)")
            log()

    log("==================================================")
    log(f" Готово. Результаты и логи в: {outdir}/")
    log("   - report.txt        (этот отчёт)")
    log("   - eeprom.raw        (если чтение удалось)")
    log("   - calibration.raw   (если чтение удалось)")
    log()
    log(f" Пришлите папку '{outdir}' разработчику,")
    log(" ОБЯЗАТЕЛЬНО указав ревизию радио и версию прошивки.")
    log("==================================================")

    tee.close()


if __name__ == "__main__":
    main()
