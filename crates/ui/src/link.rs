//! Synchronous [`DeviceLink`] over the async [`ConnectionManager`].
//!
//! Slint's window handle is `!Send` and never leaves this crate, so the host
//! drives everything on one thread. Each call enters a small current-thread
//! Tokio runtime that owns the manager and runs its notification task between
//! calls.

use std::fmt;

use openwatch_core::ble::btleplug_transport::BtleplugTransport;
use openwatch_core::ble::device_key::DeviceKey;
use openwatch_core::ble::BleTransport;
use openwatch_core::connection::{ConnectionManager, ConnectionState};
use openwatch_core::protocol::frame::Frame;
use tokio::runtime::Runtime;

use crate::bridge::{AppError, DeviceLink};

/// A [`DeviceLink`] backed by an async [`ConnectionManager<T>`].
pub struct ConnectionLink<T: BleTransport> {
    runtime: Runtime,
    manager: ConnectionManager<T>,
}

impl<T: BleTransport> fmt::Debug for ConnectionLink<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionLink").finish_non_exhaustive()
    }
}

impl<T: BleTransport> ConnectionLink<T> {
    fn connect_on(runtime: Runtime, transport: T, key: DeviceKey) -> Result<Self, AppError> {
        let mut manager = ConnectionManager::new(transport, key);
        // A failed connect is not fatal: the link still reports the resulting
        // state, and reads simply find no watch to answer them.
        let _ = runtime.block_on(manager.connect());
        Ok(Self::new(runtime, manager))
    }

    /// Wrap a manager on its runtime. The manager must already be connected
    /// for reads to receive notifications.
    pub fn new(runtime: Runtime, manager: ConnectionManager<T>) -> Self {
        Self { runtime, manager }
    }

    /// Build the runtime, connect the transport, and wrap it as a link.
    pub fn connect(transport: T, key: DeviceKey) -> Result<Self, AppError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?;
        Self::connect_on(runtime, transport, key)
    }
}

impl<T: BleTransport> DeviceLink for ConnectionLink<T> {
    fn query(&self, cmd: u8, payload: &[u8], wait_ms: u64) -> Result<Frame, AppError> {
        Ok(self
            .runtime
            .block_on(self.manager.query(cmd, payload, wait_ms))?)
    }

    fn write(&self, frame: &[u8]) -> Result<(), AppError> {
        Ok(self.runtime.block_on(self.manager.write(frame))?)
    }

    fn battery(&self) -> Option<u8> {
        self.runtime.block_on(self.manager.battery()).ok()
    }

    fn connection(&self) -> ConnectionState {
        self.manager.state()
    }
}

/// Connect the platform BLE transport and return a live link.
///
/// JNI initialisation on Android is the host's job and must happen before
/// this, because btleplug does not attach to the JVM on its own.
pub fn connect_watch(key: DeviceKey) -> Result<ConnectionLink<BtleplugTransport>, AppError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let transport = runtime.block_on(BtleplugTransport::new())?;
    ConnectionLink::connect_on(runtime, transport, key)
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use futures::{Stream, StreamExt};
    use openwatch_core::ble::BleError;
    use openwatch_core::protocol::frame::packet;

    use super::*;
    use crate::{Action, AppState};

    #[derive(Clone, Default)]
    struct Writes(Arc<Mutex<Vec<Vec<u8>>>>);

    impl Writes {
        fn all(&self) -> Vec<Vec<u8>> {
            self.0.lock().unwrap().clone()
        }
    }

    /// Transport that replays queued notifications and shares its write log.
    struct RecorderTransport {
        frames: Vec<Vec<u8>>,
        writes: Writes,
    }

    #[async_trait]
    impl BleTransport for RecorderTransport {
        async fn connect(&mut self, _key: &DeviceKey) -> Result<(), BleError> {
            Ok(())
        }
        async fn write(&self, frame: &[u8]) -> Result<(), BleError> {
            self.writes.0.lock().unwrap().push(frame.to_vec());
            Ok(())
        }
        async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError> {
            Ok(futures::stream::iter(self.frames.clone()).boxed())
        }
        async fn read_battery(&self) -> Result<u8, BleError> {
            Ok(42)
        }
        async fn disconnect(&self) -> Result<(), BleError> {
            Ok(())
        }
    }

    fn fixed_now() -> i64 {
        1_795_356_180
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn refresh_and_act_drive_the_connection_manager() {
        let writes = Writes::default();
        let steps = packet(0x2B, &[0, 0, 0, 0x1D, 0xF3]);
        let transport = RecorderTransport {
            frames: vec![steps],
            writes: writes.clone(),
        };
        let link =
            ConnectionLink::connect(transport, DeviceKey::Address("AA:BB:CC:DD:EE:FF".into()))
                .unwrap();
        assert_eq!(link.connection(), ConnectionState::Connected);
        assert_eq!(link.battery(), Some(42));

        let mut state = AppState::with_link(link, fixed_now);
        state.refresh_reads().unwrap();
        assert_eq!(state.reading("steps").unwrap().value, "7667");
        assert_eq!(state.battery, 42);
        assert!(
            writes.all().iter().any(|frame| frame.get(4) == Some(&0x2B)),
            "refresh_reads did not reach the link's query path"
        );

        state.act(Action::SetGoal(8000)).unwrap();
        assert!(
            writes.all().contains(&hex_bytes("feea20091600001f40")),
            "act did not write the exact frame through the link"
        );
    }
}
