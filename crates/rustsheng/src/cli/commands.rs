// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Subcommand handlers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rustsheng_core::client::Client;
use rustsheng_core::eeprom::{self, WriteMode};
use rustsheng_core::firmware::{self, Cpu, FirmwareImage};
use rustsheng_core::protocol;
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
        Command::ReadEeprom {
            conn,
            output,
            offset,
            size,
        } => read_eeprom(&conn, &output, offset, size),
        Command::WriteEeprom {
            conn,
            input,
            mode,
            offset,
            confirm,
        } => write_eeprom(&conn, &input, mode.into(), offset, confirm),
        Command::ReadCalibration { conn, output } => read_calibration(&conn, &output),
        Command::WriteCalibration {
            conn,
            input,
            confirm,
        } => write_calibration(&conn, &input, confirm),
        Command::Reset { conn } => reset(&conn),
        Command::ReadAdc { conn } => read_adc(&conn),
        Command::ReadRssi { conn } => read_rssi(&conn),
        Command::BootloaderInfo { conn } => bootloader_info(&conn),
        Command::Flash {
            port,
            speed,
            input,
            fw_version,
            key_number,
            dry_run,
            protocol,
            output,
            confirm,
            force_cpu,
        } => flash(
            port.as_deref(),
            speed,
            &input,
            &fw_version,
            key_number,
            dry_run,
            protocol.into(),
            output.as_deref(),
            confirm,
            force_cpu,
        ),
        Command::Unpack { input, output } => unpack(&input, output.as_deref()),
        Command::Pack {
            input,
            fw_version,
            output,
        } => pack(&input, &fw_version, output.as_deref()),
        Command::Parse { hex } => parse(&hex),
        Command::Sniffer { conn } => sniffer(&conn),
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
            .expect("valid progress template"),
    );
    pb
}

fn read_eeprom(
    opts: &ConnectionOpts,
    output: &Path,
    offset: Option<u32>,
    size: Option<u32>,
) -> Result<()> {
    let mut client = open_client(opts)?;
    let version = client.connect().context("connecting to radio")?;
    println!("Connected to firmware: {version}");

    let cpu = eeprom_cpu(&version);
    let start = offset.unwrap_or(0) as usize;
    let len = size
        .map(|s| s as usize)
        .unwrap_or(eeprom::SIZE - start.min(eeprom::SIZE));
    let limit = cpu.eeprom_limit();
    if start + len > limit {
        anyhow::bail!(
            "range {start:#06x}..{:#06x} exceeds EEPROM limit for {cpu:?} ({limit:#06x})",
            start + len,
        );
    }
    let pb = progress_bar(len, "reading");
    let data = client
        .read_region(start, len, &mut |done| pb.set_position(done as u64))
        .context("reading EEPROM")?;
    pb.finish_and_clear();
    fs::write(output, &data).with_context(|| format!("writing {}", output.display()))?;
    println!(
        "Wrote {} bytes ({:#06x}..{:#06x}) to {}",
        data.len(),
        start,
        start + data.len(),
        output.display()
    );
    Ok(())
}

fn write_eeprom(
    opts: &ConnectionOpts,
    input: &Path,
    mode: WriteMode,
    offset: Option<u32>,
    confirm: u8,
) -> Result<()> {
    let data = fs::read(input).with_context(|| format!("reading {}", input.display()))?;

    // Partial write: place the file at an explicit offset (advanced).
    if let Some(off) = offset {
        return write_eeprom_at(opts, &data, off as usize, confirm);
    }

    if mode == WriteMode::All && confirm < 1 {
        anyhow::bail!(
            "refusing to write ALL EEPROM (including calibration). \
             Re-run with --i-know-what-im-doing to proceed."
        );
    }
    if data.len() != eeprom::SIZE {
        anyhow::bail!("EEPROM file must be exactly {} bytes", eeprom::SIZE);
    }
    let mut client = open_client(opts)?;
    let version = client.connect().context("connecting to radio")?;
    println!("Connected to firmware: {version}");

    let blocks = eeprom::write_blocks(mode);
    let total: usize = blocks.iter().map(|b| b.len).sum();
    let pb = progress_bar(total, "writing");
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

/// Partial EEPROM write: place `data` at `offset`, split into block-sized writes.
fn write_eeprom_at(opts: &ConnectionOpts, data: &[u8], offset: usize, confirm: u8) -> Result<()> {
    if confirm < 1 {
        anyhow::bail!(
            "partial EEPROM write at {offset:#06x} is an advanced operation. \
             Re-run with --i-know-what-im-doing to proceed."
        );
    }
    let mut client = open_client(opts)?;
    let version = client.connect().context("connecting to radio")?;
    println!("Connected to firmware: {version}");

    let cpu = eeprom_cpu(&version);
    let limit = cpu.eeprom_limit();
    if offset + data.len() > limit {
        anyhow::bail!(
            "range {offset:#06x}..{:#06x} exceeds EEPROM limit for {cpu:?} ({limit:#06x})",
            offset + data.len(),
        );
    }
    let pb = progress_bar(data.len(), "writing");
    let mut done = 0;
    while done < data.len() {
        let len = eeprom::BLOCK.min(data.len() - done);
        let addr = (offset + done) as u16;
        client
            .write_block(addr, &data[done..done + len])
            .with_context(|| format!("writing block {addr:#06x}"))?;
        done += len;
        pb.set_position(done as u64);
    }
    pb.finish_and_clear();
    client.reset().ok();
    println!("Wrote {} bytes at {offset:#06x}", data.len());
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
        anyhow::bail!(
            "calibration file must be exactly {} bytes",
            eeprom::CALIB_SIZE
        );
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
    let (_kind, version) = client.wait_for_beacon().context("waiting for broadcast")?;
    match version {
        Some(v) => println!("Bootloader version: {v}"),
        None => println!("Flash-mode broadcast received (version not reported)"),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn flash(
    port: Option<&str>,
    speed: u32,
    input: &Path,
    fw_version: &str,
    key_number: u8,
    dry_run: bool,
    protocol: rustsheng_core::flash::FlashKind,
    output: Option<&Path>,
    confirm: u8,
    force_cpu: Option<crate::cli::ForceCpuArg>,
) -> Result<()> {
    use rustsheng_core::firmware::Cpu;
    let bytes = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let mut image = FirmwareImage::load(&bytes).context("parsing firmware image")?;
    if let Some(forced) = force_cpu {
        let cpu: Cpu = forced.into();
        println!("Overriding detected CPU {:?} -> {:?}", image.cpu, cpu);
        image.cpu = cpu;
    }
    if let Some(v) = &image.embedded_version {
        println!("Firmware file version: {v}");
    }
    let version = image
        .embedded_version
        .clone()
        .unwrap_or_else(|| fw_version.to_string());

    if dry_run {
        let seq = rustsheng_core::flash::build_sequence(
            protocol,
            &image,
            &version,
            key_number,
            rustsheng_core::flash::WRITE_ID,
        )
        .context("building flash packet sequence")?;
        let out = output
            .map(PathBuf::from)
            .unwrap_or_else(|| input.with_extension("packets.bin"));
        let mut blob = Vec::new();
        for dg in &seq {
            blob.extend_from_slice(dg);
        }
        fs::write(&out, &blob).with_context(|| format!("writing {}", out.display()))?;
        println!(
            "Dry-run: CPU {:?} (flash limit {:#x}), {} datagrams ({} bytes) written to {}",
            image.cpu,
            image.cpu.flash_limit(),
            seq.len(),
            blob.len(),
            out.display()
        );
        return Ok(());
    }

    // Live flashing (NOT validated on real hardware).
    let need = match protocol {
        rustsheng_core::flash::FlashKind::V5 => 5,
        rustsheng_core::flash::FlashKind::V2 => 3,
    };
    if confirm < need {
        anyhow::bail!(
            "flashing can permanently brick your radio and is NOT hardware-validated. \
             Re-run with at least {need} --i-know-what-im-doing flags to proceed."
        );
    }
    if image.data.len() < 50_000 && confirm < need + 2 {
        anyhow::bail!(
            "firmware image is unusually small ({} bytes); \
             re-run with {} --i-know-what-im-doing flags if intentional",
            image.data.len(),
            need + 2
        );
    }

    let port = port.context("live flashing requires --port")?;
    let conn = ConnectionOpts {
        port: port.to_string(),
        speed,
    };
    let mut client = open_client(&conn)?;
    println!("Waiting for the radio's flash-mode broadcast...");
    let (kind, boot) = client
        .wait_for_beacon()
        .context("radio is not in flash mode (power on while holding PTT)")?;
    if let Some(v) = boot {
        println!("Bootloader version: {v} ({kind:?})");
    }
    let pb = progress_bar(image.data.len(), "flashing");
    client
        .flash_firmware(&image, &version, key_number, &mut |done| {
            pb.set_position(done as u64)
        })
        .context("flashing firmware")?;
    pb.finish_and_clear();
    client.reset().ok();
    println!("Firmware flashed");
    Ok(())
}

/// `unpack`: decrypt a vendor-packed firmware image to a raw image (offline).
fn unpack(input: &Path, output: Option<&Path>) -> Result<()> {
    let bytes = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let image = FirmwareImage::load(&bytes).context("parsing firmware image")?;
    if let Some(v) = &image.embedded_version {
        println!("Embedded version: {v}");
    }
    println!(
        "Detected CPU: {:?} (flash limit {:#x})",
        image.cpu,
        image.cpu.flash_limit()
    );
    let out = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("raw"));
    fs::write(&out, &image.data).with_context(|| format!("writing {}", out.display()))?;
    println!("Unpacked {} bytes to {}", image.data.len(), out.display());
    Ok(())
}

/// `pack`: pack a raw firmware image into the vendor format (offline).
fn pack(input: &Path, version: &str, output: Option<&Path>) -> Result<()> {
    let raw = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let packed = firmware::pack(&raw, version).context("packing firmware")?;
    let out = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("packed.bin"));
    fs::write(&out, &packed).with_context(|| format!("writing {}", out.display()))?;
    println!(
        "Packed {} bytes to {} (version {version})",
        packed.len(),
        out.display()
    );
    Ok(())
}

/// `parse`: decode a hex datagram and print its clear payload (offline).
fn parse(hex: &str) -> Result<()> {
    let bytes = decode_hex(hex).context("parsing hex input")?;
    let payload = protocol::deframe(&bytes).context("decoding datagram")?;
    println!(
        "Payload ({} bytes): {}",
        payload.len(),
        encode_hex(&payload)
    );
    println!("Packet: {}", protocol::describe(&payload));
    Ok(())
}

/// `sniffer`: passively read the port and print decoded datagrams until Ctrl-C.
fn sniffer(opts: &ConnectionOpts) -> Result<()> {
    use rustsheng_core::transport::Transport;

    let mut port = serial::open(&opts.port, opts.speed)
        .with_context(|| format!("opening serial port {}", opts.port))?;
    let mut scanner = protocol::FrameScanner::new();
    let mut buf = [0u8; 256];
    println!(
        "Sniffing {} at {} baud (Ctrl-C to stop)...",
        opts.port, opts.speed
    );
    loop {
        let n = port.read_available(&mut buf).context("reading from port")?;
        if n == 0 {
            continue;
        }
        scanner.push(&buf[..n]);
        while let Some(frame) = scanner.next_frame() {
            match protocol::deframe(&frame) {
                Ok(payload) => println!("{}", protocol::describe(&payload)),
                Err(e) => println!("<undecodable datagram: {e}>"),
            }
        }
    }
}

/// Returns the CPU corresponding to a bootloader version string, defaulting to
/// `Dp32g030` when the version can't be mapped.
fn eeprom_cpu(boot_version: &str) -> Cpu {
    Cpu::from_boot_version(boot_version).unwrap_or(Cpu::Dp32g030)
}

/// Decodes a hex string (whitespace ignored) into bytes.
fn decode_hex(s: &str) -> Result<Vec<u8>> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !clean.len().is_multiple_of(2) {
        anyhow::bail!("hex string must have an even number of digits");
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).map_err(anyhow::Error::from))
        .collect()
}

/// Formats bytes as space-separated lowercase hex.
fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{b:02x}");
    }
    out
}
