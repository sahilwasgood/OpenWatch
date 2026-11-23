//! btleplug-backed implementation of [`BleTransport`].
//!
//! Notifications are delivered as a stream (not a synchronous callback): the
//! handle is obtained with `Peripheral::notifications()` and each
//! `ValueNotification`'s payload becomes one `Vec<u8>` item.

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use btleplug::api::{
    bleuuid::uuid_from_u16, Central as _, Characteristic, Manager as _, Peripheral as _,
    ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Peripheral};
use futures::{Stream, StreamExt};

use super::device_key::DeviceKey;
use super::{BleError, BleTransport};

const SERVICE_WATCH: u16 = 0xFEEA;
const CHAR_WRITE: u16 = 0xFEE2;
const CHAR_NOTIFY: u16 = 0xFEE3;
const SERVICE_BATTERY: u16 = 0x180F;
const CHAR_BATTERY: u16 = 0x2A19;

/// A BLE transport backed by a btleplug adapter.
pub struct BtleplugTransport {
    adapter: Adapter,
    peripheral: Option<Peripheral>,
    write_char: Option<Characteristic>,
    battery_char: Option<Characteristic>,
}

impl BtleplugTransport {
    /// Build a transport using the first available Bluetooth adapter.
    pub async fn new() -> Result<Self, BleError> {
        let manager = btleplug::platform::Manager::new().await?;
        let adapter = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(BleError::NoAdapter)?;
        Ok(Self::with_adapter(adapter))
    }

    /// Build a transport around a specific adapter.
    pub fn with_adapter(adapter: Adapter) -> Self {
        Self {
            adapter,
            peripheral: None,
            write_char: None,
            battery_char: None,
        }
    }
}

fn find_char_in_service(
    peripheral: &Peripheral,
    service: u16,
    short: u16,
) -> Result<Characteristic, BleError> {
    let service_uuid = uuid_from_u16(service);
    let char_uuid = uuid_from_u16(short);
    peripheral
        .services()
        .iter()
        .find(|service| service.uuid == service_uuid)
        .and_then(|service| service.characteristics.iter().find(|c| c.uuid == char_uuid))
        .cloned()
        .ok_or_else(|| BleError::CharacteristicNotFound(format!("{short:04X} on {service:04X}")))
}

/// Stopping the scan is best-effort, but a failure must not be swallowed.
fn warn_stop_scan_failure(error: btleplug::Error) {
    tracing::warn!(?error, "failed to stop BLE scan after device discovery");
}

/// BlueZ can reject `Connect` briefly after `StopDiscovery` with
/// `org.bluez.Error.InProgress`. Retry only that transient condition.
fn is_connect_in_progress(error: &btleplug::Error) -> bool {
    let message = format!("{error:?} {error}").to_lowercase();
    message.contains("inprogress") || message.contains("in progress")
}

#[async_trait]
impl BleTransport for BtleplugTransport {
    async fn connect(&mut self, key: &DeviceKey) -> Result<(), BleError> {
        let mut scan_retries = 0;
        loop {
            match self.adapter.start_scan(ScanFilter::default()).await {
                Ok(()) => break,
                Err(error) if scan_retries < 2 && is_connect_in_progress(&error) => {
                    scan_retries += 1;
                    tokio::time::sleep(Duration::from_millis(200 * scan_retries as u64)).await;
                }
                Err(error) => return Err(error.into()),
            }
        }
        let peripherals = self.adapter.peripherals().await?;

        let peripheral = match key {
            DeviceKey::Address(address) => peripherals
                .into_iter()
                .find(|p| p.address().to_string().eq_ignore_ascii_case(address)),
            DeviceKey::AdvertisedName(name) => {
                let mut match_found = None;
                for candidate in peripherals {
                    if let Ok(Some(props)) = candidate.properties().await {
                        let advertised = props
                            .advertisement_name
                            .as_deref()
                            .or(props.local_name.as_deref())
                            .unwrap_or_default();
                        if advertised.eq_ignore_ascii_case(name) {
                            match_found = Some(candidate);
                            break;
                        }
                    }
                }
                match_found
            }
        }
        .ok_or_else(|| BleError::DeviceNotFound(format!("{key:?}")))?;
        if let Err(error) = self.adapter.stop_scan().await {
            warn_stop_scan_failure(error);
        }

        let mut retries = 0;
        loop {
            match peripheral.connect().await {
                Ok(()) => break,
                Err(error) => {
                    if retries < 2 && is_connect_in_progress(&error) {
                        retries += 1;
                        tokio::time::sleep(Duration::from_millis(200 * retries as u64)).await;
                        continue;
                    }
                    return Err(error.into());
                }
            }
        }
        peripheral.discover_services().await?;

        let watch_service = uuid_from_u16(SERVICE_WATCH);
        if !peripheral
            .services()
            .iter()
            .any(|service| service.uuid == watch_service)
        {
            return Err(BleError::ServiceNotFound(format!("{SERVICE_WATCH:04X}")));
        }

        let write_char = find_char_in_service(&peripheral, SERVICE_WATCH, CHAR_WRITE)?;
        let notify_char = find_char_in_service(&peripheral, SERVICE_WATCH, CHAR_NOTIFY)?;
        let battery_char = find_char_in_service(&peripheral, SERVICE_BATTERY, CHAR_BATTERY)?;
        peripheral.subscribe(&notify_char).await?;

        self.peripheral = Some(peripheral);
        self.write_char = Some(write_char);
        self.battery_char = Some(battery_char);
        Ok(())
    }

    async fn write(&self, frame: &[u8]) -> Result<(), BleError> {
        let peripheral = self.peripheral.as_ref().ok_or(BleError::NotConnected)?;
        let char = self.write_char.as_ref().ok_or(BleError::NotConnected)?;
        peripheral
            .write(char, frame, WriteType::WithoutResponse)
            .await?;
        Ok(())
    }

    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError> {
        let peripheral = self.peripheral.as_ref().ok_or(BleError::NotConnected)?;
        let stream = peripheral.notifications().await?;
        Ok(stream.map(|notification| notification.value).boxed())
    }

    async fn read_battery(&self) -> Result<u8, BleError> {
        let peripheral = self.peripheral.as_ref().ok_or(BleError::NotConnected)?;
        let char = self.battery_char.as_ref().ok_or(BleError::NotConnected)?;
        let value = peripheral.read(char).await?;
        // A fully charged value of 100 also signals "charging" on this watch.
        Ok(value.first().copied().unwrap_or(0))
    }

    async fn disconnect(&self) -> Result<(), BleError> {
        let peripheral = self.peripheral.as_ref().ok_or(BleError::NotConnected)?;
        peripheral.disconnect().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// Minimal subscriber that counts WARN events without new dependencies.
    struct WarnCounter(Arc<Mutex<usize>>);

    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            *metadata.level() == tracing::Level::WARN
        }
        fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                *self.0.lock().unwrap() += 1;
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    #[test]
    fn in_progress_errors_are_detected_but_other_errors_are_not() {
        let transient = btleplug::Error::Other(
            "D-Bus error: org.bluez.Error.InProgress (Operation already in progress)".into(),
        );
        assert!(is_connect_in_progress(&transient));
        assert!(!is_connect_in_progress(&btleplug::Error::NotSupported(
            "test".into()
        )));
    }

    #[test]
    fn stop_scan_failure_emits_a_warning() {
        let count = Arc::new(Mutex::new(0));
        tracing::subscriber::with_default(WarnCounter(count.clone()), || {
            warn_stop_scan_failure(btleplug::Error::NotSupported("test".into()));
        });
        assert_eq!(*count.lock().unwrap(), 1);
    }
}
