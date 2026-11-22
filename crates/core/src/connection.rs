//! Connection lifecycle manager for the Fire-Boltt watch.
//!
//! Owns the BLE transport, drives the bounded reconnect-with-backoff loop,
//! serialises writes with 350 ms spacing, and matches query replies out of
//! the notification stream while ignoring spontaneous keepalives.

use std::time::Duration;

use futures::StreamExt;
use tokio::sync::{mpsc, Mutex};

use crate::ble::device_key::DeviceKey;
use crate::ble::{BleError, BleTransport};
use crate::protocol::decode::select_reply;
use crate::protocol::frame::{packet, parse, Frame};

/// Where the connection attempt cycle currently sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Idle,
    Scanning,
    Connecting,
    Connected,
    Reconnecting,
    NotFound,
}

/// Minimum gap between two consecutive writes to the watch.
const WRITE_SPACING: Duration = Duration::from_millis(350);
/// Backoff before retries: 1 s, 2 s, 4 s (one attempt per entry).
const BACKOFFS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
const NOTIFY_CAPACITY: usize = 64;

/// Drives connection, writes and queries against a single watch transport.
pub struct ConnectionManager<T: BleTransport> {
    transport: T,
    key: DeviceKey,
    state: ConnectionState,
    writer: Mutex<()>,
    notify_tx: mpsc::Sender<Frame>,
    notify_rx: Mutex<mpsc::Receiver<Frame>>,
}

impl<T: BleTransport> ConnectionManager<T> {
    pub fn new(transport: T, key: DeviceKey) -> Self {
        let (notify_tx, notify_rx) = mpsc::channel(NOTIFY_CAPACITY);
        Self {
            transport,
            key,
            state: ConnectionState::Idle,
            writer: Mutex::new(()),
            notify_tx,
            notify_rx: Mutex::new(notify_rx),
        }
    }

    /// Connect with the production 1 s / 2 s / 4 s backoff sequence.
    pub async fn connect(&mut self) -> Result<(), BleError> {
        self.connect_with_backoffs(&BACKOFFS).await
    }

    /// State machine shared by [`connect`](Self::connect) and the tests.
    /// One connection attempt per supplied backoff; sleep that long after a
    /// failed attempt before retrying.
    async fn connect_with_backoffs(&mut self, backoffs: &[Duration]) -> Result<(), BleError> {
        self.state = ConnectionState::Scanning;
        let mut last_error = None;
        for backoff in backoffs {
            self.state = ConnectionState::Connecting;
            match self.transport.connect(&self.key).await {
                Ok(()) => {
                    let stream = self.transport.subscribe().await?;
                    let tx = self.notify_tx.clone();
                    tokio::spawn(async move {
                        let mut stream = stream;
                        while let Some(bytes) = stream.next().await {
                            if let Ok(frame) = parse(&bytes) {
                                if tx.send(frame).await.is_err() {
                                    break;
                                }
                            }
                        }
                    });
                    self.state = ConnectionState::Connected;
                    return Ok(());
                }
                Err(error) => {
                    last_error = Some(error);
                    self.state = ConnectionState::Reconnecting;
                    tokio::time::sleep(*backoff).await;
                }
            }
        }
        self.state = ConnectionState::NotFound;
        Err(last_error.unwrap_or(BleError::NotConnected))
    }

    /// Write a frame, holding the writer lock through the spacing delay so
    /// two consecutive writes are never less than 350 ms apart.
    pub async fn write(&self, frame: &[u8]) -> Result<(), BleError> {
        let _guard = self.writer.lock().await;
        self.transport.write(frame).await?;
        tokio::time::sleep(WRITE_SPACING).await;
        Ok(())
    }

    /// Write a query frame, collect notifications for up to `wait_ms`, and
    /// return the frame whose command matches `cmd`.
    pub async fn query(&self, cmd: u8, payload: &[u8], wait_ms: u64) -> Result<Frame, BleError> {
        self.write(&packet(cmd, payload)).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(wait_ms);
        let mut frames = Vec::new();
        let mut rx = self.notify_rx.lock().await;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(frame)) => frames.push(frame),
                Ok(None) | Err(_) => break,
            }
        }
        drop(rx);
        select_reply(&frames, cmd).ok_or(BleError::Timeout)
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Instant;

    use async_trait::async_trait;
    use futures::{Stream, StreamExt};

    use super::*;
    use crate::ble::MockTransport;

    fn key() -> DeviceKey {
        DeviceKey::Address("AA:BB:CC:DD:EE:FF".into())
    }

    /// Transport whose connect always fails (watch asleep / not advertising).
    struct FailingTransport;

    #[async_trait]
    impl BleTransport for FailingTransport {
        async fn connect(&mut self, _key: &DeviceKey) -> Result<(), BleError> {
            Err(BleError::DeviceNotFound("asleep".into()))
        }
        async fn write(&self, _frame: &[u8]) -> Result<(), BleError> {
            Ok(())
        }
        async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError> {
            Ok(futures::stream::empty().boxed())
        }
        async fn read_battery(&self) -> Result<u8, BleError> {
            Ok(100)
        }
        async fn disconnect(&self) -> Result<(), BleError> {
            Ok(())
        }
    }

    /// Transport recording the wall-clock instant of every write.
    #[derive(Default)]
    struct TimingTransport {
        writes: Arc<StdMutex<Vec<Instant>>>,
    }

    #[async_trait]
    impl BleTransport for TimingTransport {
        async fn connect(&mut self, _key: &DeviceKey) -> Result<(), BleError> {
            Ok(())
        }
        async fn write(&self, _frame: &[u8]) -> Result<(), BleError> {
            self.writes.lock().unwrap().push(Instant::now());
            Ok(())
        }
        async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError> {
            Ok(futures::stream::empty().boxed())
        }
        async fn read_battery(&self) -> Result<u8, BleError> {
            Ok(100)
        }
        async fn disconnect(&self) -> Result<(), BleError> {
            Ok(())
        }
    }

    #[test]
    fn default_backoffs_are_one_two_four_seconds() {
        assert_eq!(
            BACKOFFS,
            [
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
            ]
        );
    }

    #[tokio::test]
    async fn connect_fails_cleanly_with_not_found_after_three_attempts() {
        let mut manager = ConnectionManager::new(FailingTransport, key());
        // Zero backoffs keep the state-machine check instant; the public
        // connect() is the path that applies the real 1 s / 2 s / 4 s sleeps.
        let result = manager.connect_with_backoffs(&[Duration::ZERO; 3]).await;
        assert!(result.is_err());
        assert_eq!(manager.state(), ConnectionState::NotFound);
    }

    #[tokio::test]
    async fn query_skips_keepalive_and_returns_matching_reply() {
        let frames = vec![packet(0xF9, &[]), packet(0x2E, &[1])];
        let mut manager = ConnectionManager::new(MockTransport::new(frames), key());
        manager.connect().await.unwrap();
        let reply = manager.query(0x2E, &[], 50).await.unwrap();
        assert_eq!(
            reply,
            Frame {
                cmd: 0x2E,
                payload: vec![1],
            }
        );
    }

    #[tokio::test]
    async fn consecutive_writes_are_spaced_at_least_350ms() {
        let writes = Arc::new(StdMutex::new(Vec::new()));
        let transport = TimingTransport {
            writes: writes.clone(),
        };
        let manager = ConnectionManager::new(transport, key());
        manager.write(&[1, 2, 3]).await.unwrap();
        manager.write(&[4, 5, 6]).await.unwrap();

        let recorded = writes.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert!(recorded[1].duration_since(recorded[0]) >= WRITE_SPACING);
    }
}
