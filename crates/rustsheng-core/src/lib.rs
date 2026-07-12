//! `rustsheng-core`: hardware-independent core for programming Quansheng UV-K5
//! radios over serial. Contains the wire protocol, high-level operations, and a
//! [`transport::Transport`] abstraction so the logic can be exercised without
//! real hardware.

pub mod client;
pub mod eeprom;
pub mod firmware;
pub mod protocol;
pub mod transport;
