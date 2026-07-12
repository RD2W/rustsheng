//! Byte-level transport abstraction. The core talks to any type implementing
//! [`Transport`]; a real serial port lives in [`serial`] (feature `serial`),
//! and [`mock::MockTransport`] backs the tests.

use std::time::Duration;

pub mod mock;
#[cfg(feature = "serial")]
pub mod serial;

/// Errors returned by a [`Transport`].
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Underlying I/O failure (message preserved for diagnostics).
    #[error("transport I/O error: {0}")]
    Io(String),
    /// The requested bytes did not arrive before the timeout elapsed.
    #[error("transport timeout")]
    Timeout,
}

/// A discovered serial port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    /// OS-specific port name (e.g. `/dev/ttyUSB0` or `COM3`).
    pub name: String,
}

/// A bidirectional byte stream to the radio.
pub trait Transport {
    /// Writes the entire buffer.
    fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Reads exactly `buf.len()` bytes or fails with [`TransportError::Timeout`].
    fn read_exact_timeout(
        &mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError>;

    /// Discards any buffered input.
    fn flush_input(&mut self) -> Result<(), TransportError>;
}
