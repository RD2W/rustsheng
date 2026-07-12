// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF) and k5prog-win (OneOfEleven).

//! Datagram framing: `AB CD len_lo len_hi <payload> crc_lo crc_hi DC BA`.
//! Only the payload and its CRC are obfuscated; the header, length, and footer
//! are sent in the clear.

use crate::protocol::{crc16_xmodem, xor_payload};

/// Errors produced while decoding a datagram received from the radio.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtocolError {
    /// Fewer bytes than the minimum framing overhead (8 bytes).
    #[error("datagram too short")]
    TooShort,
    /// The `AB CD` start marker was missing.
    #[error("bad header (expected AB CD)")]
    BadHeader,
    /// The `DC BA` end marker was missing.
    #[error("bad footer (expected DC BA)")]
    BadFooter,
    /// The declared payload length does not match the buffer size.
    #[error("length mismatch: declared {declared}, buffer implies {actual}")]
    LengthMismatch { declared: usize, actual: usize },
    /// The CRC did not match and was not the radio's `0xFFFF` sentinel.
    #[error("bad CRC: got {got:#06x}, want {want:#06x}")]
    BadCrc { got: u16, want: u16 },
}

/// Builds an obfuscated datagram from a clear `payload`.
pub fn frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut out = Vec::with_capacity(len + 8);
    out.push(0xAB);
    out.push(0xCD);
    out.push((len & 0xff) as u8);
    out.push(((len >> 8) & 0xff) as u8);

    let mut body = Vec::with_capacity(len + 2);
    body.extend_from_slice(payload);
    let crc = crc16_xmodem(payload);
    body.push((crc & 0xff) as u8);
    body.push(((crc >> 8) & 0xff) as u8);
    xor_payload(&mut body);

    out.extend_from_slice(&body);
    out.push(0xDC);
    out.push(0xBA);
    out
}

/// Decodes an obfuscated datagram and returns the clear payload (CRC removed).
///
/// Accepts the radio's `0xFFFF` CRC sentinel as valid (the radio does not
/// compute a real CRC on its replies).
pub fn deframe(datagram: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if datagram.len() < 8 {
        return Err(ProtocolError::TooShort);
    }
    if datagram[0] != 0xAB || datagram[1] != 0xCD {
        return Err(ProtocolError::BadHeader);
    }
    let declared = u16::from_le_bytes([datagram[2], datagram[3]]) as usize;
    let expected_total = 4 + declared + 2 + 2;
    if datagram.len() != expected_total {
        return Err(ProtocolError::LengthMismatch {
            declared,
            actual: datagram.len().saturating_sub(8),
        });
    }
    if datagram[expected_total - 2] != 0xDC || datagram[expected_total - 1] != 0xBA {
        return Err(ProtocolError::BadFooter);
    }

    // payload + 2 CRC bytes
    let mut body = datagram[4..4 + declared + 2].to_vec();
    xor_payload(&mut body);
    let crc_got = u16::from_le_bytes([body[declared], body[declared + 1]]);
    let payload = body[..declared].to_vec();
    let crc_want = crc16_xmodem(&payload);
    if crc_got != 0xFFFF && crc_got != crc_want {
        return Err(ProtocolError::BadCrc {
            got: crc_got,
            want: crc_want,
        });
    }
    Ok(payload)
}

/// Largest datagram payload the scanner will accept (guards against garbage on
/// the wire declaring an absurd length).
const MAX_SCAN_PAYLOAD: usize = 512;

/// Extracts complete obfuscated datagrams from a byte stream, resynchronising on
/// the `AB CD` start marker. Feed bytes with [`push`](Self::push) and pull each
/// framed datagram with [`next_frame`](Self::next_frame). Used by the sniffer.
#[derive(Debug, Default)]
pub struct FrameScanner {
    buf: Vec<u8>,
}

impl FrameScanner {
    /// Creates an empty scanner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends received bytes to the internal buffer.
    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Returns the next complete datagram (`AB CD .. DC BA`), or `None` if more
    /// bytes are needed. Leading garbage and malformed frames are discarded.
    pub fn next_frame(&mut self) -> Option<Vec<u8>> {
        loop {
            // Need at least the 8-byte framing overhead to decide anything.
            if self.buf.len() < 8 {
                return None;
            }
            // Resynchronise to the AB CD start marker.
            if self.buf[0] != 0xAB || self.buf[1] != 0xCD {
                match self.buf.iter().skip(1).position(|&b| b == 0xAB) {
                    Some(pos) => {
                        self.buf.drain(..pos + 1);
                        continue;
                    }
                    None => {
                        self.buf.clear();
                        return None;
                    }
                }
            }
            let declared = u16::from_le_bytes([self.buf[2], self.buf[3]]) as usize;
            if declared > MAX_SCAN_PAYLOAD {
                // Implausible length: drop the start byte and resynchronise.
                self.buf.drain(..1);
                continue;
            }
            let total = 4 + declared + 2 + 2;
            if self.buf.len() < total {
                return None;
            }
            if self.buf[total - 2] != 0xDC || self.buf[total - 1] != 0xBA {
                // Bad footer: not a real frame here; drop one byte and retry.
                self.buf.drain(..1);
                continue;
            }
            return Some(self.buf.drain(..total).collect());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let payload = vec![0x14, 0x05, 0x04, 0x00, 0x6a, 0x39, 0x57, 0x64];
        let datagram = frame(&payload);
        assert_eq!(&datagram[0..2], &[0xAB, 0xCD]);
        assert_eq!(&datagram[datagram.len() - 2..], &[0xDC, 0xBA]);
        assert_eq!(deframe(&datagram).unwrap(), payload);
    }

    #[test]
    fn accepts_radio_ffff_crc() {
        // Build a datagram whose CRC field decodes to 0xFFFF, like the radio sends.
        let payload = vec![0x18u8, 0x05, 0x00, 0x00];
        let mut out = vec![0xAB, 0xCD, payload.len() as u8, 0x00];
        let mut body = payload.clone();
        body.push(0xFF);
        body.push(0xFF);
        xor_payload(&mut body);
        out.extend_from_slice(&body);
        out.extend_from_slice(&[0xDC, 0xBA]);
        assert_eq!(deframe(&out).unwrap(), payload);
    }

    #[test]
    fn rejects_bad_header() {
        let mut datagram = frame(&[1, 2, 3]);
        datagram[0] = 0x00;
        assert_eq!(deframe(&datagram), Err(ProtocolError::BadHeader));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut datagram = frame(&[1, 2, 3]);
        // Corrupt an obfuscated payload byte so the CRC check fails.
        datagram[4] ^= 0xFF;
        assert!(matches!(
            deframe(&datagram),
            Err(ProtocolError::BadCrc { .. })
        ));
    }

    #[test]
    fn scanner_extracts_frames_and_resyncs() {
        let a = frame(&[0x14, 0x05, 0x04, 0x00]);
        let b = frame(&[0x1b, 0x05, 0x08, 0x00]);
        let mut scanner = FrameScanner::new();
        // Leading garbage before the first frame, then the two frames back to back.
        scanner.push(&[0x00, 0xff, 0x12]);
        scanner.push(&a);
        scanner.push(&b);
        let f1 = scanner.next_frame().expect("first frame");
        let f2 = scanner.next_frame().expect("second frame");
        assert_eq!(deframe(&f1).unwrap(), vec![0x14, 0x05, 0x04, 0x00]);
        assert_eq!(deframe(&f2).unwrap(), vec![0x1b, 0x05, 0x08, 0x00]);
        assert!(scanner.next_frame().is_none());
    }

    #[test]
    fn scanner_waits_for_complete_frame() {
        let datagram = frame(&[0x29, 0x05, 0x00, 0x00]);
        let mut scanner = FrameScanner::new();
        scanner.push(&datagram[..datagram.len() - 1]); // one byte short
        assert!(scanner.next_frame().is_none());
        scanner.push(&datagram[datagram.len() - 1..]); // final byte
        assert!(scanner.next_frame().is_some());
    }
}
