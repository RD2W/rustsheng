#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
#
# rustsheng: firmware FLASH with backup + dry-run (cross-platform Python).
#
# !!! ОПАСНО: ПРОШИВКА МОЖЕТ НЕОБРАТИМО ПРЕВРАТИТЬ РАДИО В КИРПИЧ !!!
# !!! Прошивка НЕ проверена на реальном железе. Используйте на свой страх. !!!
#
# Скрипт делает всё максимально осторожно:
#   1) (обычный режим) бэкап EEPROM и калибровки;
#   2) (офлайн) --dry-run: строит поток пакетов в файл, ничего не пишет в радио;
#   3) требует явного подтверждения 'FLASH';
#   4) (режим прошивки) реальная прошивка с нужным числом --i-know-what-im-doing.
#
# Использование:
#   ./rustsheng_flash.py --port /dev/ttyUSB0 --firmware firmware.bin --protocol v2
#   ./rustsheng_flash.py --port COM3 --firmware firmware.bin --force-cpu py32f071

import argparse
import datetime
import platform
import shutil
import subprocess
import sys
from pathlib import Path


CONFIRMS_V2 = 3
CONFIRMS_V5 = 5


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
        description="rustsheng firmware FLASH with backup + dry-run (cross-platform)"
    )
    parser.add_argument(
        "--port", "-p", required=True, help="Последовательный порт (напр. /dev/ttyUSB0 или COM3)"
    )
    parser.add_argument(
        "--firmware", "-f", required=True, metavar="FIRMWARE.BIN",
        help="Путь к файлу прошивки",
    )
    parser.add_argument(
        "--protocol",
        choices=["v2", "v5"],
        default="v2",
        help="Протокол загрузчика (v2 или v5, по умолчанию: v2)",
    )
    parser.add_argument(
        "--force-cpu",
        choices=["dp32g030", "py32f030", "py32f071"],
        default=None,
        help="Явно указать CPU (для нестандартных сборок, если автоопределение ошибается)",
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

    fw_path = Path(args.firmware)
    if not fw_path.is_file():
        sys.exit(f"ОШИБКА: файл прошивки не найден: {args.firmware}")

    num_confirms = CONFIRMS_V5 if args.protocol == "v5" else CONFIRMS_V2
    confirm_flags = ["--i-know-what-im-doing"] * num_confirms

    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    outdir = Path(f"rustsheng-flash-{stamp}")
    outdir.mkdir(parents=True, exist_ok=True)
    log_path = outdir / "report.txt"
    ee_backup = outdir / "eeprom-backup.raw"
    cal_backup = outdir / "calibration-backup.raw"
    packets = outdir / "packets.bin"

    tee = TeeLogger(log_path)

    def log(msg: str = "") -> None:
        tee.write(msg + "\n")

    def fail(msg: str) -> None:
        log(f"!!! ОШИБКА: {msg}")
        log(f"Бэкапы (если созданы) в: {outdir}/")
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
        log(f"(код возврата: {result.returncode})")
        log()
        return result.returncode

    os_info = platform.uname()
    log("==================================================")
    log(f" rustsheng FLASH (backup + dry-run) — {stamp}")
    log(f" Порт:      {args.port}")
    log(f" Прошивка:  {args.firmware}")
    log(f" Протокол:  {args.protocol}")
    log(f" Бинарник:  {bin_path}")
    log(f" ОС:        {os_info.system} {os_info.release} {os_info.version} {os_info.machine}")
    log(f" Python:    {sys.version}")
    log("==================================================")
    log()
    log("!!! ПРОШИВКА МОЖЕТ НАВСЕГДА ИСПОРТИТЬ РАДИО (КИРПИЧ) !!!")
    log("!!! Функция НЕ проверена на реальном железе.          !!!")
    log()
    log(">>> УКАЖИТЕ В ОТЧЁТЕ РЕВИЗИЮ РАДИО (V1/V2/V3/K1) И ПРОШИВКУ <<<")
    log()

    # --- 1) Бэкап (обычный режим) -----------------------------------------
    log("########## ШАГ 1: БЭКАП ##########")
    log("Радио должно быть в ОБЫЧНОМ режиме (PTT НЕ зажат).")
    log("Радио в обычном режиме? Enter — делать бэкап, 's' — пропустить:")
    a = sys.stdin.readline().strip()
    log(f"> {a}")
    if a.lower() != "s":
        rc = run("Бэкап EEPROM", ["read-eeprom", "-p", args.port, "-o", str(ee_backup)])
        if rc != 0:
            fail("не удалось прочитать EEPROM-бэкап")
        rc = run("Бэкап калибровки", ["read-calibration", "-p", args.port, "-o", str(cal_backup)])
        if rc != 0:
            log("(калибровку прочитать не удалось — продолжаем)")
        log(f"Бэкапы сохранены в {outdir}/")
    else:
        log("Бэкап пропущен пользователем (НЕ рекомендуется).")
    log()

    # --- 2) Dry-run (офлайн, без записи) ----------------------------------
    log("########## ШАГ 2: DRY-RUN (без записи) ##########")
    dry_run_args = ["flash", "-i", str(fw_path), "--dry-run", "--protocol", args.protocol, "-o", str(packets)]
    if args.force_cpu:
        dry_run_args.extend(["--force-cpu", args.force_cpu])
    rc = run("Dry-run", dry_run_args)
    if rc != 0:
        fail("dry-run не удался — образ невалиден?")
    log(f"Поток пакетов построен: {packets}")
    log()

    # --- 3) Подтверждение -------------------------------------------------
    log("########## ШАГ 3: ПОДТВЕРЖДЕНИЕ ##########")
    log("Дальше будет РЕАЛЬНАЯ прошивка. Отмена — что угодно кроме 'FLASH'.")
    log("Введите заглавными 'FLASH' для продолжения:")
    confirm = sys.stdin.readline().strip()
    log(f"> {confirm}")
    if confirm != "FLASH":
        log(f"Прошивка отменена. Бэкапы и dry-run в {outdir}/")
        tee.close()
        sys.exit(0)
    log()

    # --- 4) Реальная прошивка (режим прошивки) -----------------------------
    log("########## ШАГ 4: ПРОШИВКА ##########")
    log("ПЕРЕВЕДИТЕ РАДИО В РЕЖИМ ПРОШИВКИ: выключить, зажать PTT + верхнюю")
    log("боковую кнопку, включить (загорится подсветка), затем нажмите Enter.")
    log("Радио в режиме прошивки? Enter — прошивать:")
    sys.stdin.readline()
    log("> (Enter)")

    flash_args = ["flash", "-p", args.port, "-i", str(fw_path)]
    if args.protocol == "v5":
        flash_args.extend(["--key-number", "0"])
    if args.force_cpu:
        flash_args.extend(["--force-cpu", args.force_cpu])
    flash_args.extend(confirm_flags)

    log(f"$ {bin_path} {fmt_args(flash_args + ['<{0} флагов подтверждения>'.format(num_confirms)])}")
    result = subprocess.run(
        [bin_path, *flash_args],
        capture_output=True,
        text=True,
    )
    log(result.stdout.rstrip())
    if result.stderr:
        log(result.stderr.rstrip())
    if result.returncode != 0:
        fail("прошивка завершилась ошибкой")
    log()

    log("==================================================")
    log(f" Готово. Результаты в: {outdir}/")
    log("   - report.txt              (этот отчёт)")
    log("   - eeprom-backup.raw       (бэкап EEPROM — ХРАНИТЕ!)")
    log("   - calibration-backup.raw  (бэкап калибровки)")
    log("   - packets.bin             (dry-run поток)")
    log()
    log(f" Пришлите папку '{outdir}' разработчику,")
    log(" указав ревизию радио и версию прошивки.")
    log()
    log(" ВОССТАНОВЛЕНИЕ EEPROM (после успешной прошивки, в обычном режиме):")
    log(f"   {bin_path} write-eeprom -p {args.port} -i {ee_backup} --mode original")
    log("==================================================")

    tee.close()


if __name__ == "__main__":
    main()
