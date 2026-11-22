//! Rust-side state that feeds the generated Slint window.
//!
//! The generated window type stays inside this crate: Slint 1.19 cannot export
//! it across crate boundaries. Hosts drive the UI through [`crate::run`].

use openwatch_core::connection::ConnectionState;
use slint::SharedString;
use thiserror::Error;

use crate::AppWindow;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("UI error: {0}")]
    Ui(#[from] slint::PlatformError),
}

/// State shown in the shell. Later tasks populate these fields from the local
/// store and decoded replies, keeping this module free of I/O.
#[derive(Debug, Clone)]
pub struct AppState {
    pub device_name: String,
    pub battery: u8,
    pub connection: ConnectionState,
    /// Pre-formatted UTC label, e.g. `2026-11-22 14:03 UTC`.
    pub last_sync: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            device_name: "Fire-Boltt 039".into(),
            battery: 0,
            connection: ConnectionState::Idle,
            last_sync: None,
        }
    }
}

/// Accessibility switches. Slint 1.19 has no OS detection for these, so the
/// shell exposes them explicitly; the short cross-fade is always used.
#[derive(Debug, Clone, Copy, Default)]
pub struct UiOptions {
    pub reduced_transparency: bool,
    pub high_contrast: bool,
}

fn connection_label(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Idle => "idle",
        ConnectionState::Scanning => "scanning",
        ConnectionState::Connecting => "connecting",
        ConnectionState::Connected => "connected",
        ConnectionState::Reconnecting => "reconnecting",
        ConnectionState::NotFound => "not found",
    }
}

fn last_sync_label(value: Option<&str>) -> &str {
    value.unwrap_or("never")
}

impl AppState {
    pub(crate) fn apply_to(&self, window: &AppWindow, options: &UiOptions) {
        window.set_device_name(SharedString::from(self.device_name.as_str()));
        window.set_battery(self.battery as i32);
        window.set_connection_state(SharedString::from(connection_label(self.connection)));
        window.set_last_sync(SharedString::from(last_sync_label(
            self.last_sync.as_deref(),
        )));
        window.set_reduced_transparency(options.reduced_transparency);
        window.set_high_contrast(options.high_contrast);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_states_have_human_labels() {
        assert_eq!(connection_label(ConnectionState::Idle), "idle");
        assert_eq!(connection_label(ConnectionState::Connected), "connected");
        assert_eq!(connection_label(ConnectionState::NotFound), "not found");
    }

    #[test]
    fn missing_last_sync_reads_never() {
        assert_eq!(last_sync_label(None), "never");
        assert_eq!(
            last_sync_label(Some("2026-11-22 14:03 UTC")),
            "2026-11-22 14:03 UTC"
        );
    }
}
