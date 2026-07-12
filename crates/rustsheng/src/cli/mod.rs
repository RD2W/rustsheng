// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! Command-line interface definition (clap derive) and the connection options
//! shared by radio subcommands.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

pub mod commands;

/// Program-and-flash tool for Quansheng UV-K5 radios.
#[derive(Debug, Parser)]
#[command(
    name = "rustsheng",
    version,
    author,
    about,
    help_template = "\
{name} {version}
{author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}"
)]
pub struct Cli {
    /// Increase verbosity (repeat for more: -v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

/// Serial connection options shared by every radio subcommand.
#[derive(Debug, Args)]
pub struct ConnectionOpts {
    /// Serial port (e.g. /dev/ttyUSB0 or COM3).
    #[arg(short, long)]
    pub port: String,

    /// Serial speed in baud (the UV-K5 uses 38400).
    #[arg(short, long, default_value_t = 38400)]
    pub speed: u32,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// List available serial ports.
    ScanPorts,

    /// Read the EEPROM to a file (full image, or a range with --offset/--size).
    #[command(visible_alias = "r")]
    ReadEeprom {
        #[command(flatten)]
        conn: ConnectionOpts,
        /// Output file.
        #[arg(short, long, default_value = "k5_eeprom.raw")]
        output: PathBuf,
        /// Start offset (decimal or 0x-hex; default 0).
        #[arg(long, value_parser = parse_u32)]
        offset: Option<u32>,
        /// Number of bytes to read (decimal or 0x-hex; default: to end of EEPROM).
        #[arg(long, value_parser = parse_u32)]
        size: Option<u32>,
    },

    /// Write the EEPROM from a file (full image, or a range with --offset).
    #[command(visible_alias = "w")]
    WriteEeprom {
        #[command(flatten)]
        conn: ConnectionOpts,
        /// Input file (full-image mode: exactly 0x2000 bytes).
        #[arg(short, long)]
        input: PathBuf,
        /// Which blocks to write (full-image mode only).
        #[arg(long, value_enum, default_value_t = WriteModeArg::Original)]
        mode: WriteModeArg,
        /// Write the file at this offset instead of a full image (decimal or 0x-hex).
        #[arg(long, value_parser = parse_u32)]
        offset: Option<u32>,
        /// Confirm dangerous operations (repeat to raise the confidence level).
        #[arg(long = "i-know-what-im-doing", action = clap::ArgAction::Count)]
        confirm: u8,
    },

    /// Read the calibration region (0x1e00..0x2000) to a file.
    ReadCalibration {
        #[command(flatten)]
        conn: ConnectionOpts,
        #[arg(short, long, default_value = "k5_calibration.raw")]
        output: PathBuf,
    },

    /// Write the calibration region from a file (0x200 bytes).
    WriteCalibration {
        #[command(flatten)]
        conn: ConnectionOpts,
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long = "i-know-what-im-doing", action = clap::ArgAction::Count)]
        confirm: u8,
    },

    /// Reboot the radio.
    Reset {
        #[command(flatten)]
        conn: ConnectionOpts,
    },

    /// Read the battery ADC value.
    ReadAdc {
        #[command(flatten)]
        conn: ConnectionOpts,
    },

    /// Read RSSI / noise / glitch.
    ReadRssi {
        #[command(flatten)]
        conn: ConnectionOpts,
    },

    /// Wait for the flash-mode broadcast and print the bootloader version.
    BootloaderInfo {
        #[command(flatten)]
        conn: ConnectionOpts,
    },

    /// Flash a firmware image (raw or vendor-encrypted .bin).
    #[command(visible_alias = "F")]
    Flash {
        #[command(flatten)]
        conn: ConnectionOpts,
        /// Firmware file.
        #[arg(short, long)]
        input: PathBuf,
        /// Version string sent to the bootloader.
        #[arg(short = 'M', long, default_value = "*.01.23")]
        fw_version: String,
        #[arg(long = "i-know-what-im-doing", action = clap::ArgAction::Count)]
        confirm: u8,
    },

    /// Decrypt a vendor-packed firmware image to a raw image (offline).
    Unpack {
        /// Packed (or already-raw) firmware file.
        #[arg(short, long)]
        input: PathBuf,
        /// Output raw image (default: <input>.raw).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Pack a raw firmware image into the vendor format (offline).
    Pack {
        /// Raw firmware file.
        #[arg(short, long)]
        input: PathBuf,
        /// Version string embedded at 0x2000.
        #[arg(short = 'M', long, default_value = "*.01.23")]
        fw_version: String,
        /// Output packed image (default: <input>.packed.bin).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Decode a hex datagram and print its payload (offline).
    Parse {
        /// Datagram as a hex string (e.g. `abcd0800...dcba`).
        hex: String,
    },

    /// Passively sniff the serial port and print decoded datagrams (Ctrl-C to stop).
    Sniffer {
        #[command(flatten)]
        conn: ConnectionOpts,
    },
}

/// Parses an unsigned integer, accepting `0x`/`0X` hex or decimal.
fn parse_u32(s: &str) -> Result<u32, std::num::ParseIntError> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => s.parse(),
    }
}

/// CLI mirror of [`rustsheng_core::eeprom::WriteMode`].
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum WriteModeArg {
    /// Vendor block set/order.
    Original,
    /// Everything up to 0x1d00.
    Most,
    /// The whole EEPROM ("brick" mode).
    All,
}

impl From<WriteModeArg> for rustsheng_core::eeprom::WriteMode {
    fn from(m: WriteModeArg) -> Self {
        match m {
            WriteModeArg::Original => Self::Original,
            WriteModeArg::Most => Self::Most,
            WriteModeArg::All => Self::All,
        }
    }
}
