//! High-level radio operations. A [`Client`] wraps any [`Transport`] and
//! implements the request/response exchanges the radio understands.

use std::time::Duration;

use crate::protocol::frame::ProtocolError;
use crate::protocol::{commands, deframe, frame};
use crate::transport::{Transport, TransportError};

/// Default per-transaction read timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(10_000);
/// Number of hello attempts made by [`Client::connect`].
pub const HELLO_TRIES: usize = 10;

/// Errors returned by [`Client`] operations.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Transport-level failure.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// Malformed datagram.
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    /// The 4-byte datagram header was not `AB CD .. 00`.
    #[error("bad datagram magic")]
    BadMagic,
    /// The radio is in firmware-update mode but a normal-mode op was requested.
    #[error("radio is in firmware flash mode (power on without PTT for normal mode)")]
    RadioInFlashMode,
    /// A response did not match what the command expected.
    #[error("unexpected response: {0}")]
    Unexpected(String),
    /// The radio never confirmed a write/flash.
    #[error("operation not confirmed: {0}")]
    NotConfirmed(String),
    /// The radio did not answer the handshake.
    #[error("radio not detected")]
    NotDetected,
}

/// Battery ADC reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdcInfo {
    /// Raw ADC value; voltage requires per-radio calibration data.
    pub raw: u16,
}

/// RSSI / noise / glitch reading.
#[derive(Debug, Clone, Copy)]
pub struct RssiInfo {
    /// Raw 9-bit RSSI value.
    pub rssi_raw: u16,
    /// RSSI converted to dBm (`raw / 2 - 160`).
    pub dbm: f32,
    /// 7-bit noise value.
    pub noise: u8,
    /// Glitch counter.
    pub glitch: u8,
}

/// A connected (or connectable) radio.
pub struct Client<T: Transport> {
    transport: T,
    timeout: Duration,
}

impl<T: Transport> Client<T> {
    /// Wraps `transport` with the default timeout.
    pub fn new(transport: T) -> Self {
        Self::with_timeout(transport, DEFAULT_TIMEOUT)
    }

    /// Wraps `transport` with an explicit read `timeout`.
    pub fn with_timeout(transport: T, timeout: Duration) -> Self {
        Self { transport, timeout }
    }

    /// Sends one command payload and returns the clear response payload.
    fn transaction(&mut self, payload: &[u8]) -> Result<Vec<u8>, ClientError> {
        self.transport.flush_input()?;
        self.transport.write_all(&frame(payload))?;
        self.read_response()
    }

    /// Reads and decodes a single datagram from the transport.
    fn read_response(&mut self) -> Result<Vec<u8>, ClientError> {
        let mut header = [0u8; 4];
        self.transport.read_exact_timeout(&mut header, self.timeout)?;
        if header[0] != 0xAB || header[1] != 0xCD {
            return Err(ClientError::BadMagic);
        }
        let data_len = u16::from_le_bytes([header[2], header[3]]) as usize;
        let mut rest = vec![0u8; data_len + 2 + 2];
        self.transport.read_exact_timeout(&mut rest, self.timeout)?;
        let mut full = Vec::with_capacity(4 + rest.len());
        full.extend_from_slice(&header);
        full.extend_from_slice(&rest);
        Ok(deframe(&full)?)
    }

    /// Performs the handshake and returns the firmware version string.
    ///
    /// Returns [`ClientError::RadioInFlashMode`] if the radio replies with a
    /// `0x18` flash-mode broadcast instead of a `0x15` hello reply.
    pub fn hello(&mut self) -> Result<String, ClientError> {
        let reply = self.transaction(&commands::hello())?;
        if reply.first() == Some(&0x18) {
            return Err(ClientError::RadioInFlashMode);
        }
        if reply.len() < 20 || reply[0] != 0x15 || reply[1] != 0x05 {
            return Err(ClientError::Unexpected(format!(
                "hello reply cmd={:#04x}",
                reply.first().copied().unwrap_or(0)
            )));
        }
        let version_bytes = &reply[4..20];
        let end = version_bytes.iter().position(|&b| b == 0).unwrap_or(16);
        Ok(String::from_utf8_lossy(&version_bytes[..end]).into_owned())
    }

    /// Retries [`hello`](Self::hello) up to [`HELLO_TRIES`] times.
    pub fn connect(&mut self) -> Result<String, ClientError> {
        let mut last = ClientError::NotDetected;
        for _ in 0..HELLO_TRIES {
            match self.hello() {
                Ok(v) => return Ok(v),
                Err(ClientError::RadioInFlashMode) => return Err(ClientError::RadioInFlashMode),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    /// Reads a single EEPROM block (`len` <= 0x80) at `addr`.
    pub fn read_eeprom(&mut self, addr: u16, len: u8) -> Result<Vec<u8>, ClientError> {
        let reply = self.transaction(&commands::read_eeprom(addr, len))?;
        if reply.first() == Some(&0x18) {
            return Err(ClientError::RadioInFlashMode);
        }
        // reply: 1c 05 .. .. addr_lo addr_hi .. .. <data...>
        if reply.len() < 8 + len as usize
            || reply[0] != 0x1c
            || reply[4] != (addr & 0xff) as u8
            || reply[5] != (addr >> 8) as u8
        {
            return Err(ClientError::Unexpected(format!(
                "read reply for addr {addr:#06x}"
            )));
        }
        Ok(reply[8..8 + len as usize].to_vec())
    }

    /// Reads `size` bytes starting at `start` in [`crate::eeprom::BLOCK`] chunks,
    /// reporting cumulative bytes read via `progress`.
    pub fn read_region(
        &mut self,
        start: usize,
        size: usize,
        progress: &mut dyn FnMut(usize),
    ) -> Result<Vec<u8>, ClientError> {
        use crate::eeprom::BLOCK;
        let mut out = Vec::with_capacity(size);
        let mut done = 0;
        while done < size {
            let len = BLOCK.min(size - done);
            let addr = (start + done) as u16;
            out.extend_from_slice(&self.read_eeprom(addr, len as u8)?);
            done += len;
            progress(done);
        }
        Ok(out)
    }

    /// Reads the entire EEPROM image.
    pub fn read_eeprom_full(
        &mut self,
        progress: &mut dyn FnMut(usize),
    ) -> Result<Vec<u8>, ClientError> {
        self.read_region(0, crate::eeprom::SIZE, progress)
    }

    /// Writes one EEPROM block and verifies the radio's `0x1e` confirmation.
    pub fn write_block(&mut self, addr: u16, data: &[u8]) -> Result<(), ClientError> {
        let reply = self.transaction(&commands::write_eeprom(addr, data))?;
        if reply.first() == Some(&0x18) {
            return Err(ClientError::RadioInFlashMode);
        }
        if reply.len() < 6
            || reply[0] != 0x1e
            || reply[4] != (addr & 0xff) as u8
            || reply[5] != (addr >> 8) as u8
        {
            return Err(ClientError::NotConfirmed(format!("write at {addr:#06x}")));
        }
        Ok(())
    }

    /// Reads the battery ADC value (reply `0x2a`).
    pub fn read_adc(&mut self) -> Result<AdcInfo, ClientError> {
        let reply = self.transaction(&commands::read_adc())?;
        if reply.first() == Some(&0x18) {
            return Err(ClientError::RadioInFlashMode);
        }
        if reply.len() < 8 || reply[0] != 0x2a || reply[1] != 0x05 {
            return Err(ClientError::Unexpected("adc reply".into()));
        }
        Ok(AdcInfo {
            raw: u16::from_le_bytes([reply[4], reply[5]]),
        })
    }

    /// Reads RSSI/noise/glitch (reply `0x28`).
    pub fn read_rssi(&mut self) -> Result<RssiInfo, ClientError> {
        let reply = self.transaction(&commands::read_rssi())?;
        if reply.first() == Some(&0x18) {
            return Err(ClientError::RadioInFlashMode);
        }
        if reply.len() < 8 || reply[0] != 0x28 || reply[1] != 0x05 {
            return Err(ClientError::Unexpected("rssi reply".into()));
        }
        let rssi_raw = u16::from_le_bytes([reply[4], reply[5]]) & 0x01FF;
        Ok(RssiInfo {
            rssi_raw,
            dbm: (rssi_raw as f32 / 2.0) - 160.0,
            noise: reply[6] & 0x7F,
            glitch: reply[7],
        })
    }

    /// Reboots the radio (fire-and-forget; no reply expected).
    pub fn reset(&mut self) -> Result<(), ClientError> {
        self.transport.flush_input()?;
        self.transport.write_all(&frame(&commands::reset()))?;
        Ok(())
    }

    /// Waits for the radio's `0x18` flash-mode broadcast and returns the
    /// bootloader version string when the packet carries one.
    pub fn wait_flash_broadcast(&mut self) -> Result<Option<String>, ClientError> {
        let reply = self.read_response()?;
        if reply.len() < 2 || reply[0] != 0x18 || reply[1] != 0x05 {
            return Err(ClientError::Unexpected("flash broadcast".into()));
        }
        if reply.len() >= 36 {
            let mut end = 0x14;
            while end < reply.len() && reply[end].is_ascii_graphic() {
                end += 1;
            }
            let v = String::from_utf8_lossy(&reply[0x14..end]).into_owned();
            return Ok((!v.is_empty()).then_some(v));
        }
        Ok(None)
    }

    /// Sends the firmware version to the bootloader (flash mode only).
    pub fn send_flash_version(&mut self, version: &str) -> Result<(), ClientError> {
        let _ = self.transaction(&commands::flash_version(version))?;
        Ok(())
    }

    /// Writes one flash block and verifies the `0x1a` confirmation, ignoring
    /// repeated `0x18` broadcasts (up to 5 reply attempts).
    pub fn write_flash_block(
        &mut self,
        offset: u16,
        data: &[u8],
        firmware_size: usize,
    ) -> Result<(), ClientError> {
        self.transport.flush_input()?;
        self.transport
            .write_all(&frame(&commands::write_flash(offset, data, firmware_size)))?;
        for _ in 0..5 {
            let reply = match self.read_response() {
                Ok(r) => r,
                Err(_) => continue,
            };
            if reply.first() == Some(&0x18) {
                continue;
            }
            if reply.len() >= 10
                && reply[0] == 0x1a
                && reply[8] == (offset >> 8) as u8
                && reply[9] == (offset & 0xff) as u8
            {
                return Ok(());
            }
        }
        Err(ClientError::NotConfirmed(format!(
            "flash block at {offset:#06x}"
        )))
    }

    /// Flashes an entire image: version handshake, then block-by-block write.
    /// `progress` receives the cumulative byte count.
    pub fn flash_firmware(
        &mut self,
        image: &crate::firmware::FirmwareImage,
        version: &str,
        progress: &mut dyn FnMut(usize),
    ) -> Result<(), ClientError> {
        self.send_flash_version(version)?;
        let size = image.data.len();
        for (offset, chunk) in image.blocks() {
            self.write_flash_block(offset, chunk, size)?;
            progress((offset as usize) + chunk.len());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockTransport;

    /// Builds a radio reply datagram from a clear payload (CRC = 0xFFFF).
    fn radio_reply(payload: &[u8]) -> Vec<u8> {
        use crate::protocol::xor_payload;
        let mut out = vec![0xAB, 0xCD, payload.len() as u8, 0x00];
        let mut body = payload.to_vec();
        body.push(0xFF);
        body.push(0xFF);
        xor_payload(&mut body);
        out.extend_from_slice(&body);
        out.extend_from_slice(&[0xDC, 0xBA]);
        out
    }

    #[test]
    fn hello_parses_version() {
        // 0x15 reply with "k5_2.01.26" in the 16-byte version field.
        let mut payload = vec![0x15u8, 0x05, 0x24, 0x00];
        let mut ver = [0u8; 16];
        ver[..10].copy_from_slice(b"k5_2.01.26");
        payload.extend_from_slice(&ver);
        payload.extend_from_slice(&[0u8; 20]); // flags + challenge padding
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&payload)]));
        assert_eq!(client.hello().unwrap(), "k5_2.01.26");
    }

    #[test]
    fn hello_detects_flash_mode() {
        let payload = vec![0x18u8, 0x05, 0x20, 0x00];
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&payload)]));
        assert!(matches!(client.hello(), Err(ClientError::RadioInFlashMode)));
    }

    #[test]
    fn read_eeprom_returns_block_data() {
        let mut payload = vec![0x1c, 0x05, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00];
        payload.extend_from_slice(&[0x42; 0x80]);
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&payload)]));
        let data = client.read_eeprom(0x0080, 0x80).unwrap();
        assert_eq!(data, vec![0x42; 0x80]);
    }

    #[test]
    fn write_block_requires_confirmation() {
        let good = vec![0x1e, 0x05, 0x00, 0x00, 0x10, 0x00];
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&good)]));
        assert!(client.write_block(0x0010, &[0u8; 0x10]).is_ok());
    }

    #[test]
    fn read_adc_decodes_raw() {
        // 2A 05 04 00 AC 07 00 00 -> 0x07AC
        let payload = vec![0x2a, 0x05, 0x04, 0x00, 0xac, 0x07, 0x00, 0x00];
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&payload)]));
        assert_eq!(client.read_adc().unwrap().raw, 0x07AC);
    }

    #[test]
    fn read_rssi_decodes_fields() {
        // 28 05 04 00 8E 00 50 42
        let payload = vec![0x28, 0x05, 0x04, 0x00, 0x8e, 0x00, 0x50, 0x42];
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&payload)]));
        let r = client.read_rssi().unwrap();
        assert_eq!(r.rssi_raw, 0x008E);
        assert_eq!(r.noise, 0x50);
        assert_eq!(r.glitch, 0x42);
    }

    #[test]
    fn write_flash_block_accepts_confirmation() {
        let payload = vec![
            0x1a, 0x05, 0x08, 0x00, 0x8a, 0x8d, 0x9f, 0x1d, 0x01, 0x00, 0x00, 0x00,
        ];
        let mut client = Client::new(MockTransport::new(vec![radio_reply(&payload)]));
        assert!(client.write_flash_block(0x0100, &[0u8; 0x100], 0x0800).is_ok());
    }

    #[test]
    fn wait_flash_broadcast_reads_version() {
        let mut payload = vec![0u8; 36];
        payload[0] = 0x18;
        payload[1] = 0x05;
        payload[0x14..0x14 + 7].copy_from_slice(b"2.00.06");
        let mut client = Client::new(MockTransport::new_preloaded(radio_reply(&payload)));
        assert_eq!(client.wait_flash_broadcast().unwrap().as_deref(), Some("2.00.06"));
    }
}
