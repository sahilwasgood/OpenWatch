//! Thin desktop host. The UI and its generated Slint types live in openwatch-ui.

use openwatch_core::ble::device_key::DeviceKey;
use openwatch_ui::{connect_watch, run_live, AppError, UiOptions};

/// Configured watch; macOS resolves this to the advertised name instead.
const WATCH_ADDRESS: &str = "67:31:E4:DD:50:7A";

/// Entry point used by the desktop binary and by tests.
pub fn run() -> Result<(), AppError> {
    let key = if cfg!(target_os = "macos") {
        DeviceKey::AdvertisedName(openwatch_core::ble::device_key::WATCH_ADVERTISED_NAME.into())
    } else {
        DeviceKey::Address(WATCH_ADDRESS.into())
    };
    run_live(connect_watch(key)?, UiOptions::default())
}

fn main() -> Result<(), AppError> {
    run()
}
