//! In-memory [`Transport`] for tests. Each write pops the next scripted
//! response (if any) and appends it to the internal read buffer, mimicking a
//! request/response exchange with the radio.

use std::collections::VecDeque;
use std::time::Duration;

use super::{Transport, TransportError};

/// Scripted transport: records writes and serves canned responses.
pub struct MockTransport {
    written: Vec<Vec<u8>>,
    read_buf: VecDeque<u8>,
    script: VecDeque<Vec<u8>>,
}

impl MockTransport {
    /// Creates a transport that appends `scripted_responses[i]` to its read
    /// buffer on the `i`-th write.
    pub fn new(scripted_responses: Vec<Vec<u8>>) -> Self {
        Self {
            written: Vec::new(),
            read_buf: VecDeque::new(),
            script: scripted_responses.into(),
        }
    }

    /// Creates a transport with `data` already in the read buffer (for
    /// testing broadcast reads that do not trigger a write).
    pub fn new_preloaded(data: Vec<u8>) -> Self {
        Self {
            written: Vec::new(),
            read_buf: VecDeque::from(data),
            script: VecDeque::new(),
        }
    }

    /// Returns every buffer passed to [`Transport::write_all`], in order.
    pub fn written(&self) -> &[Vec<u8>] {
        &self.written
    }
}

impl Transport for MockTransport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.written.push(data.to_vec());
        if let Some(resp) = self.script.pop_front() {
            self.read_buf.extend(resp);
        }
        Ok(())
    }

    fn read_exact_timeout(
        &mut self,
        buf: &mut [u8],
        _timeout: Duration,
    ) -> Result<usize, TransportError> {
        if self.read_buf.len() < buf.len() {
            return Err(TransportError::Timeout);
        }
        for slot in buf.iter_mut() {
            *slot = self.read_buf.pop_front().unwrap();
        }
        Ok(buf.len())
    }

    fn flush_input(&mut self) -> Result<(), TransportError> {
        self.read_buf.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_scripted_response_after_write() {
        let mut t = MockTransport::new(vec![vec![1, 2, 3, 4]]);
        t.write_all(&[0xAA]).unwrap();
        let mut buf = [0u8; 4];
        assert_eq!(t.read_exact_timeout(&mut buf, Duration::ZERO).unwrap(), 4);
        assert_eq!(buf, [1, 2, 3, 4]);
        assert_eq!(t.written(), &[vec![0xAA]]);
    }

    #[test]
    fn times_out_without_data() {
        let mut t = MockTransport::new(vec![]);
        let mut buf = [0u8; 2];
        assert!(matches!(
            t.read_exact_timeout(&mut buf, Duration::ZERO),
            Err(TransportError::Timeout)
        ));
    }
}
