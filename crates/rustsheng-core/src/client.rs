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
}
