#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
#
# rustsheng: EEPROM WRITE round-trip test (cross-platform Python).
#
# !!! ВНИМАНИЕ: ЭТОТ СКРИПТ ПИШЕТ В EEPROM РАДИО !!!
#
# Он безопасен настолько, насколько это возможно для теста записи:
#   1) читает полный дамп EEPROM в бэкап-файл;
#   2) записывает ТОТ ЖЕ дамп обратно (mode=original) — данные не меняются;
#   3) перечитывает EEPROM и сверяет с бэкапом.
# Записываются те же самые байты, поэтому настройки радио НЕ меняются.
# Бэкап всегда сохраняется — им можно восстановиться при любой проблеме.
#
# ТЕМ НЕ МЕНЕЕ запись во flash-память несёт риск. Используйте на свой страх.
#
# Использование:
#   ./rustsheng_test_write.py --port /dev/ttyUSB0
#   ./rustsheng_test_write.py --port COM3 --bin ./rustsheng.exe

import argparse
import datetime
import platform
import shutil
import subprocess
import sys
from pathlib import Path


MAX_DIFFS = 40


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


def main() -> None:
    if sys.platform == "win32":
        sys.stdout.reconfigure(encoding="utf-8")       # type: ignore[union-attr]
        sys.stderr.reconfigure(encoding="utf-8")       # type: ignore[union-attr]

    default_bin = "rustsheng.exe" if sys.platform == "win32" else "rustsheng"

    parser = argparse.ArgumentParser(
        description="rustsheng EEPROM WRITE round-trip test (cross-platform)"
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
    outdir = Path(f"rustsheng-write-test-{stamp}")
    outdir.mkdir(parents=True, exist_ok=True)
    log_path = outdir / "report.txt"
    backup_path = outdir / "eeprom-backup.raw"
    verify_path = outdir / "eeprom-verify.raw"

    tee = TeeLogger(log_path)

    def log(msg: str = "") -> None:
        tee.write(msg + "\n")

    def fail(msg: str) -> None:
        log(f"!!! ОШИБКА: {msg}")
        log(f"Бэкап (если создан) лежит в: {backup_path}")
        tee.close()
        sys.exit(1)

    def run(title: str, cmd_args: list[str]) -> int:
        log(f"### {title}")
        log(f"$ {bin_path} {fmt_args(cmd_args)}")
        result = subprocess.run(
            [bin_path, *cmd_args],
            capture_output=True,
            text=True,
        )
        log(result.stdout.rstrip())
        if result.stderr:
            log(result.stderr.rstrip())
        return result.returncode

    os_info = platform.uname()
    log("==================================================")
    log(f" rustsheng EEPROM WRITE round-trip — {stamp}")
    log(f" Порт:     {args.port}")
    log(f" Бинарник: {bin_path}")
    log(f" ОС:       {os_info.system} {os_info.release} {os_info.version} {os_info.machine}")
    log(f" Python:   {sys.version}")
    log("==================================================")
    log()
    log("!!! ЭТОТ ТЕСТ ПИШЕТ В EEPROM. Записывается тот же дамп,")
    log("    что был прочитан, поэтому настройки НЕ изменятся.")
    log(f"    Бэкап будет сохранён в: {backup_path}")
    log()
    log(">>> УКАЖИТЕ В ОТЧЁТЕ РЕВИЗИЮ РАДИО (V1/V2/V3/K1) И ПРОШИВКУ <<<")
    log()

    log("Продолжить? Введите 'yes' чтобы согласиться на запись:")
    ans = sys.stdin.readline().strip()
    log(f"> {ans}")
    if ans != "yes":
        log("Отменено пользователем.")
        tee.close()
        sys.exit(0)
    log()

    # --- 1) Бэкап ---------------------------------------------------------
    if run("Шаг 1/4: чтение бэкапа EEPROM", ["read-eeprom", "-p", args.port, "-o", str(backup_path)]) != 0:
        fail("не удалось прочитать бэкап")
    if not backup_path.is_file() or backup_path.stat().st_size == 0:
        fail("бэкап пустой")
    log()

    # --- 2) Запись того же дампа обратно ----------------------------------
    log("### Шаг 2/4: запись того же дампа обратно (mode=original)")
    cmd = ["write-eeprom", "-p", args.port, "-i", str(backup_path), "--mode", "original"]
    log(f"$ {bin_path} {fmt_args(cmd)}")
    result = subprocess.run([bin_path, *cmd], capture_output=True, text=True)
    log(result.stdout.rstrip())
    if result.stderr:
        log(result.stderr.rstrip())
    if result.returncode != 0:
        fail("запись не удалась (бэкап цел, можно восстановить)")
    log()

    # --- 3) Перечитывание для проверки -----------------------------------
    if run("Шаг 3/4: повторное чтение для проверки", ["read-eeprom", "-p", args.port, "-o", str(verify_path)]) != 0:
        fail("не удалось перечитать для проверки")
    log()

    # --- 4) Сравнение -----------------------------------------------------
    log("### Шаг 4/4: сравнение бэкапа и повторного чтения")
    backup_data = backup_path.read_bytes()
    verify_data = verify_path.read_bytes()

    if len(backup_data) != len(verify_data):
        log(f"РАСХОЖДЕНИЕ: разный размер: backup={len(backup_data)} verify={len(verify_data)}")
        log(f"Для восстановления настроек:")
        log(f"  {bin_path} write-eeprom -p {args.port} -i {backup_path} --mode original")
        log()
        tee.close()
        return

    diffs: list[str] = []
    min_len = min(len(backup_data), len(verify_data))
    for i in range(min_len):
        if backup_data[i] != verify_data[i]:
            if len(diffs) < MAX_DIFFS:
                diffs.append(
                    f"offset 0x{i:04X}: backup=0x{backup_data[i]:02X} verify=0x{verify_data[i]:02X}"
                )

    if not diffs:
        log("УСПЕХ: EEPROM после записи совпадает с бэкапом (round-trip OK).")
    else:
        log("РАСХОЖДЕНИЕ: перечитанный EEPROM отличается от бэкапа!")
        for line in diffs:
            log(line)
        if len([1 for b, v in zip(backup_data, verify_data) if b != v]) > MAX_DIFFS:
            log(f"... (показано первых {MAX_DIFFS} различий)")
        log("Для восстановления настроек:")
        log(f"  {bin_path} write-eeprom -p {args.port} -i {backup_path} --mode original")
    log()

    log("==================================================")
    log(f" Готово. Результаты в: {outdir}/")
    log("   - report.txt          (этот отчёт)")
    log("   - eeprom-backup.raw   (исходный дамп — ХРАНИТЕ!)")
    log("   - eeprom-verify.raw   (дамп после записи)")
    log()
    log(f" Пришлите папку '{outdir}' разработчику,")
    log(" указав ревизию радио и версию прошивки.")
    log()
    log(" ВОССТАНОВЛЕНИЕ (если что-то пошло не так):")
    log(f"   {bin_path} write-eeprom -p {args.port} -i {backup_path} --mode original")
    log("==================================================")

    tee.close()


if __name__ == "__main__":
    main()
