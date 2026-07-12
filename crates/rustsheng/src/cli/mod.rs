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

    /// Read the full EEPROM to a file.
    #[command(visible_alias = "r")]
    ReadEeprom {
        #[command(flatten)]
        conn: ConnectionOpts,
        /// Output file.
        #[arg(short, long, default_value = "k5_eeprom.raw")]
        output: PathBuf,
    },

    /// Write the EEPROM from a file.
    #[command(visible_alias = "w")]
    WriteEeprom {
        #[command(flatten)]
        conn: ConnectionOpts,
        /// Input file (must be exactly 0x2000 bytes).
        #[arg(short, long)]
        input: PathBuf,
        /// Which blocks to write.
        #[arg(long, value_enum, default_value_t = WriteModeArg::Original)]
        mode: WriteModeArg,
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
