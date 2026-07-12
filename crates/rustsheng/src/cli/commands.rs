//! Subcommand handlers.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rustsheng_core::client::Client;
use rustsheng_core::eeprom::{self, WriteMode};
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
        other => {
            let _ = other;
            anyhow::bail!("not yet implemented");
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
