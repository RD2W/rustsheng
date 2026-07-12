//! Subcommand handlers.

use anyhow::{Context, Result};
use rustsheng_core::client::Client;
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
