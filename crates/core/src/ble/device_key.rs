//! Device identity resolution for the Fire-Boltt watch.
//!
//! macOS does not expose stable peripheral MAC addresses, so on that platform
//! the watch is identified by its advertised name instead.

/// How a watch is identified during discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceKey {
    Address(String),
    AdvertisedName(String),
}

/// The advertised name macOS matches against.
pub const WATCH_ADVERTISED_NAME: &str = "FireBoltt";

/// Resolve a configured identifier into the platform-appropriate key.
///
/// macOS does not expose stable MAC addresses for BLE peripherals, so the
/// configured value is ignored there and the advertised name is used instead.
pub fn resolve_key(platform: &str, configured: &str) -> DeviceKey {
    if platform == "macos" {
        DeviceKey::AdvertisedName(WATCH_ADVERTISED_NAME.to_string())
    } else {
        DeviceKey::Address(configured.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_key_linux_uses_configured_address() {
        assert_eq!(
            resolve_key("linux", "67:31:E4:DD:50:7A"),
            DeviceKey::Address("67:31:E4:DD:50:7A".into())
        );
    }

    #[test]
    fn resolve_key_macos_uses_advertised_name() {
        assert_eq!(
            resolve_key("macos", "67:31:E4:DD:50:7A"),
            DeviceKey::AdvertisedName("FireBoltt".into())
        );
    }
}
