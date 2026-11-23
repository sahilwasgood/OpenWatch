//! Thin desktop host. The UI and its generated Slint types live in openwatch-ui.

use std::path::PathBuf;

use openwatch_core::ble::device_key::DeviceKey;
use openwatch_core::store::Store;
use openwatch_ui::{connect_watch, run_live, AppError, UiOptions};

/// Configured watch; macOS resolves this to the advertised name instead.
const WATCH_ADDRESS: &str = "67:31:E4:DD:50:7A";

/// Per-user writable directory for the local SQLite store. No account, no cloud.
fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                let root = PathBuf::from(home);
                #[cfg(target_os = "macos")]
                {
                    root.join("Library/Application Support")
                }
                #[cfg(not(target_os = "macos"))]
                {
                    root.join(".local/share")
                }
            })
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let dir = base.join("openwatch");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Entry point used by the desktop binary and by tests.
pub fn run() -> Result<(), AppError> {
    let key = if cfg!(target_os = "macos") {
        DeviceKey::AdvertisedName(openwatch_core::ble::device_key::WATCH_ADVERTISED_NAME.into())
    } else {
        DeviceKey::Address(WATCH_ADDRESS.into())
    };
    let db = data_dir().join("openwatch.sqlite3");
    let store = Store::open(db.to_string_lossy().as_ref())?;
    run_live(connect_watch(key)?, store, UiOptions::default())
}

fn main() -> Result<(), AppError> {
    run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_is_under_a_writable_user_path() {
        let dir = data_dir();
        assert!(dir.ends_with("openwatch"), "{dir:?}");
        assert!(dir.is_dir(), "store directory was not created: {dir:?}");
    }
}
