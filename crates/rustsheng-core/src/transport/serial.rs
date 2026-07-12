//! Real serial-port transport built on the `serialport` crate. Compiled only
//! with the `serial` feature.

use std::io::{Read, Write};
use std::time::Duration;

use super::{PortInfo, Transport, TransportError};

/// A [`Transport`] backed by an OS serial port.
pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
}

/// Opens `port` at `baud` with 8 data bits, no parity, one stop bit.
pub fn open(port: &str, baud: u32) -> Result<SerialTransport, TransportError> {
    let port = serialport::new(port, baud)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| TransportError::Io(e.to_string()))?;
    Ok(SerialTransport { port })
}

/// Lists the serial ports visible to the OS.
pub fn scan_ports() -> Vec<PortInfo> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| PortInfo { name: p.port_name })
        .collect()
}

impl Transport for SerialTransport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.port
            .write_all(data)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        self.port
            .flush()
            .map_err(|e| TransportError::Io(e.to_string()))
    }

    fn read_exact_timeout(
        &mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        // Accumulate bytes until `buf` is full or the overall deadline passes.
        let deadline = std::time::Instant::now() + timeout;
        let mut filled = 0;
        while filled < buf.len() {
            if std::time::Instant::now() >= deadline {
                return Err(TransportError::Timeout);
            }
            match self.port.read(&mut buf[filled..]) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                Ok(n) => filled += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(e) => return Err(TransportError::Io(e.to_string())),
            }
        }
        Ok(filled)
    }

    fn flush_input(&mut self) -> Result<(), TransportError> {
        self.port
            .clear(serialport::ClearBuffer::Input)
            .map_err(|e| TransportError::Io(e.to_string()))
    }
}
