//! Bluetooth Low Energy transport abstraction for the Fire-Boltt watch.
//!
//! The GATT layout: service `FEEA`, write characteristic `FEE2`, notify
//! characteristic `FEE3`, file-transfer characteristic `FEE6`, live-steps
//! characteristic `FEE1`, and battery level `2A19` on service `180F`.
//! A battery value of 100 means "charging".

pub mod btleplug_transport;
pub mod device_key;

use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use thiserror::Error;

use self::device_key::DeviceKey;

/// Errors returned by BLE transports.
#[derive(Debug, Error)]
pub enum BleError {
    #[error("BLE backend error: {0}")]
    Backend(#[from] btleplug::Error),
    #[error("no Bluetooth adapter found")]
    NoAdapter,
    #[error("device not found: {0}")]
    DeviceNotFound(String),
    #[error("not connected to a device")]
    NotConnected,
    #[error("GATT service not found: {0}")]
    ServiceNotFound(String),
    #[error("GATT characteristic not found: {0}")]
    CharacteristicNotFound(String),
    #[error("no matching reply received")]
    Timeout,
}

/// Async BLE operations against a connected watch.
#[async_trait]
pub trait BleTransport {
    async fn connect(&mut self, key: &DeviceKey) -> Result<(), BleError>;
    async fn write(&self, frame: &[u8]) -> Result<(), BleError>;
    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError>;
    async fn read_battery(&self) -> Result<u8, BleError>;
    async fn disconnect(&self) -> Result<(), BleError>;
}

/// In-memory transport for tests: records writes and replays queued frames.
pub struct MockTransport {
    writes: std::sync::Mutex<Vec<Vec<u8>>>,
    frames: Vec<Vec<u8>>,
}

impl MockTransport {
    pub fn new(frames: Vec<Vec<u8>>) -> Self {
        Self {
            writes: std::sync::Mutex::new(Vec::new()),
            frames,
        }
    }

    /// Every frame passed to `write`, in order.
    pub fn writes(&self) -> Vec<Vec<u8>> {
        self.writes.lock().unwrap().clone()
    }
}

#[async_trait]
impl BleTransport for MockTransport {
    async fn connect(&mut self, _key: &DeviceKey) -> Result<(), BleError> {
        Ok(())
    }

    async fn write(&self, frame: &[u8]) -> Result<(), BleError> {
        self.writes.lock().unwrap().push(frame.to_vec());
        Ok(())
    }

    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError> {
        Ok(futures::stream::iter(self.frames.clone()).boxed())
    }

    async fn read_battery(&self) -> Result<u8, BleError> {
        Ok(100)
    }

    async fn disconnect(&self) -> Result<(), BleError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn mock_write_records_exact_frame() {
        let transport = MockTransport::new(vec![]);
        let frame = vec![0xF9, 0x01, 0x02];
        futures::executor::block_on(transport.write(&frame)).unwrap();
        assert_eq!(transport.writes(), vec![frame]);
    }

    #[test]
    fn mock_subscribe_yields_queued_frames_including_f9() {
        let frames = vec![vec![0xF9], vec![0x01, 0x02, 0x03]];
        let transport = MockTransport::new(frames.clone());
        let stream = futures::executor::block_on(transport.subscribe()).unwrap();
        let got = futures::executor::block_on(stream.collect::<Vec<_>>());
        assert_eq!(got, frames);
        assert!(got.iter().any(|frame| frame.contains(&0xF9)));
    }
}
