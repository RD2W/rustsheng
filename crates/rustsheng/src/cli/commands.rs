//! Subcommand handlers.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rustsheng_core::client::Client;
use rustsheng_core::eeprom::{self, WriteMode};
use rustsheng_core::firmware::FirmwareImage;
use rustsheng_core::transport::serial::{self, SerialTransport};

use super::{Command, ConnectionOpts};

/// Opens a serial connection and returns a [`Client`].
pub fn open_client(opts: &ConnectionOpts) -> Result<Client<SerialTransport>> {
    let transport = serial::open(&opts.port, opts.speed)
        .with_context(|| format!("opening serial port {}", opts.port))?;
    Ok(Client::new(transport))
}

/// Executes a parsed command.
pub fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::ScanPorts => scan_ports(),
        Command::ReadEeprom { conn, output } => read_eeprom(&conn, &output),
        Command::WriteEeprom { conn, input, mode, confirm } => {
            write_eeprom(&conn, &input, mode.into(), confirm)
        }
        Command::ReadCalibration { conn, output } => read_calibration(&conn, &output),
        Command::WriteCalibration { conn, input, confirm } => {
            write_calibration(&conn, &input, confirm)
        }
        Command::Reset { conn } => reset(&conn),
        Command::ReadAdc { conn } => read_adc(&conn),
        Command::ReadRssi { conn } => read_rssi(&conn),
        Command::BootloaderInfo { conn } => bootloader_info(&conn),
        Command::Flash { conn, input, fw_version, confirm } => {
            flash(&conn, &input, &fw_version, confirm)
        }
    }
}

/// `scan-ports`: print discovered serial ports.
fn scan_ports() -> Result<()> {
    let ports = serial::scan_ports();
    if ports.is_empty() {
        println!("No serial ports found");
    } else {
        println!("Available serial ports:");
        for p in ports {
            println!("  {}", p.name);
        }
    }
    Ok(())
}

fn progress_bar(total: usize, label: &str) -> ProgressBar {
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template(&format!("{label} {{bar:40}} {{bytes}}/{{total_bytes}}"))
            .unwrap(),
    );
    pb
}

fn read_eeprom(opts: &ConnectionOpts, output: &Path) -> Result<()> {
    let mut client = open_client(opts)?;
    let version = client.connect().context("connecting to radio")?;
    println!("Connected to firmware: {version}");
    let pb = progress_bar(eeprom::SIZE, "reading");
    let data = client
        .read_eeprom_full(&mut |done| pb.set_position(done as u64))
        .context("reading EEPROM")?;
    pb.finish_and_clear();
    fs::write(output, &data).with_context(|| format!("writing {}", output.display()))?;
    println!("Wrote {} bytes to {}", data.len(), output.display());
    Ok(())
}

fn write_eeprom(opts: &ConnectionOpts, input: &Path, mode: WriteMode, confirm: u8) -> Result<()> {
    if mode == WriteMode::All && confirm < 1 {
        anyhow::bail!(
            "refusing to write ALL EEPROM (including calibration). \
             Re-run with --i-know-what-im-doing to proceed."
        );
    }
    let data = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    if data.len() != eeprom::SIZE {
        anyhow::bail!("EEPROM file must be exactly {} bytes", eeprom::SIZE);
    }
    let mut client = open_client(opts)?;
    let version = client.connect().context("connecting to radio")?;
    println!("Connected to firmware: {version}");

    let blocks = eeprom::write_blocks(mode);
    let pb = progress_bar(eeprom::SIZE, "writing");
    let mut done = 0;
    for b in blocks {
        client
            .write_block(b.offset as u16, &data[b.offset..b.offset + b.len])
            .with_context(|| format!("writing block {:#06x}", b.offset))?;
        done += b.len;
        pb.set_position(done as u64);
    }
    pb.finish_and_clear();
    client.reset().ok();
    println!("EEPROM written");
    Ok(())
}

fn read_calibration(opts: &ConnectionOpts, output: &Path) -> Result<()> {
    let mut client = open_client(opts)?;
    let version = client.connect().context("connecting to radio")?;
    println!("Connected to firmware: {version}");
    let pb = progress_bar(eeprom::CALIB_SIZE, "reading");
    let data = client
        .read_region(eeprom::CALIB_START, eeprom::CALIB_SIZE, &mut |d| {
            pb.set_position(d as u64)
        })
        .context("reading calibration")?;
    pb.finish_and_clear();
    fs::write(output, &data).with_context(|| format!("writing {}", output.display()))?;
    println!("Wrote {} bytes to {}", data.len(), output.display());
    Ok(())
}

fn write_calibration(opts: &ConnectionOpts, input: &Path, confirm: u8) -> Result<()> {
    if confirm < 1 {
        anyhow::bail!(
            "writing calibration can affect radio performance. \
             Re-run with --i-know-what-im-doing to proceed."
        );
    }
    let data = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    if data.len() != eeprom::CALIB_SIZE {
        anyhow::bail!("calibration file must be exactly {} bytes", eeprom::CALIB_SIZE);
    }
    let mut client = open_client(opts)?;
    client.connect().context("connecting to radio")?;
    let pb = progress_bar(eeprom::CALIB_SIZE, "writing");
    let mut done = 0;
    for b in eeprom::calibration_blocks() {
        let local = b.offset - eeprom::CALIB_START;
        client
            .write_block(b.offset as u16, &data[local..local + b.len])
            .with_context(|| format!("writing calibration block {:#06x}", b.offset))?;
        done += b.len;
        pb.set_position(done as u64);
    }
    pb.finish_and_clear();
    client.reset().ok();
    println!("Calibration written");
    Ok(())
}

fn reset(opts: &ConnectionOpts) -> Result<()> {
    let mut client = open_client(opts)?;
    client.connect().context("connecting to radio")?;
    client.reset().context("resetting radio")?;
    println!("Reset command sent");
    Ok(())
}

fn read_adc(opts: &ConnectionOpts) -> Result<()> {
    let mut client = open_client(opts)?;
    client.connect().context("connecting to radio")?;
    let adc = client.read_adc().context("reading ADC")?;
    println!("Battery ADC: {} (0x{:04X})", adc.raw, adc.raw);
    Ok(())
}

fn read_rssi(opts: &ConnectionOpts) -> Result<()> {
    let mut client = open_client(opts)?;
    client.connect().context("connecting to radio")?;
    let r = client.read_rssi().context("reading RSSI")?;
    println!(
        "RSSI {} ({:.1} dBm), Noise {}, Glitch {}",
        r.rssi_raw, r.dbm, r.noise, r.glitch
    );
    Ok(())
}

fn bootloader_info(opts: &ConnectionOpts) -> Result<()> {
    let mut client = open_client(opts)?;
    println!("Waiting for the radio's flash-mode broadcast...");
    match client.wait_flash_broadcast().context("waiting for broadcast")? {
        Some(v) => println!("Bootloader version: {v}"),
        None => println!("Flash-mode broadcast received (version not reported)"),
    }
    Ok(())
}

fn flash(opts: &ConnectionOpts, input: &Path, fw_version: &str, confirm: u8) -> Result<()> {
    if confirm < 3 {
        anyhow::bail!(
            "flashing firmware can permanently brick your radio. \
             Re-run with -M and at least three --i-know-what-im-doing flags to proceed."
        );
    }
    let bytes = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let image = FirmwareImage::load(&bytes).context("parsing firmware image")?;
    if let Some(v) = &image.embedded_version {
        println!("Firmware file version: {v}");
    }
    if image.data.len() < 50_000 && confirm < 5 {
        anyhow::bail!(
            "firmware image is unusually small ({} bytes); \
             re-run with five --i-know-what-im-doing flags if this is intentional",
            image.data.len()
        );
    }

    let mut client = open_client(opts)?;
    println!("Waiting for the radio's flash-mode broadcast...");
    let boot = client
        .wait_flash_broadcast()
        .context("radio is not in flash mode (power on while holding PTT)")?;
    if let Some(v) = boot {
        println!("Bootloader version: {v}");
    }

    let version = image.embedded_version.clone().unwrap_or_else(|| fw_version.to_string());
    let pb = progress_bar(image.data.len(), "flashing");
    client
        .flash_firmware(&image, &version, &mut |done| pb.set_position(done as u64))
        .context("flashing firmware")?;
    pb.finish_and_clear();
    client.reset().ok();
    println!("Firmware flashed");
    Ok(())
}
