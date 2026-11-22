//! Shared Slint UI for OpenWatch. Both the desktop and Android hosts build
//! against this library so the `.slint` documents are compiled exactly once.
//!
//! The generated window type never leaves this crate (Slint 1.19 issue #7060):
//! hosts call [`run`] or [`run_with`] with ordinary Rust state instead.

slint::include_modules!();

pub mod bridge;

pub use bridge::{AppError, AppState, UiOptions};

/// Build, populate and run the shell with supplied state and options.
pub fn run_with(state: AppState, options: UiOptions) -> Result<(), AppError> {
    let window = AppWindow::new()?;
    state.apply_to(&window, &options);
    window.run()?;
    Ok(())
}

/// Run the shell with default state and options.
pub fn run() -> Result<(), AppError> {
    run_with(AppState::default(), UiOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    #[test]
    fn ui_exposes_four_navigation_destinations() {
        let window = AppWindow::new().expect("app-window.slint must generate a usable window");
        let names: Vec<String> = window
            .get_navigation_destinations()
            .iter()
            .map(|name| name.to_string())
            .collect();
        assert_eq!(names, ["Today", "Device", "Workbench", "Data"]);

        // One window per test process: winit cannot recreate its event loop.
        let set = |state: &str| {
            window.set_connection_state(slint::SharedString::from(state));
            window.get_status_color()
        };
        assert_eq!(set("connected"), slint::Color::from_rgb_u8(0x30, 0xD1, 0x58));
        for state in ["reconnecting", "scanning", "connecting"] {
            assert_eq!(set(state), slint::Color::from_rgb_u8(0xFF, 0x9F, 0x0A), "{state}");
        }
        assert_eq!(set("not found"), slint::Color::from_rgb_u8(0xFF, 0x45, 0x3A));
    }

    #[test]
    fn theme_defines_accessibility_fallbacks() {
        let theme = include_str!("../ui/theme.slint");
        assert!(
            theme.contains("solid-surface"),
            "reduced-transparency solid surface token missing"
        );
        assert!(
            theme.contains("reduced-motion-crossfade"),
            "reduced-motion cross-fade token missing"
        );
        assert!(
            theme.contains("high-contrast-border"),
            "high-contrast border token missing"
        );
    }

    #[test]
    fn shell_applies_content_and_status_tokens() {
        let shell = include_str!("../ui/app-window.slint");
        for token in [
            "content-surface",
            "status-good",
            "status-warn",
            "status-bad",
            "motion-press",
        ] {
            assert!(shell.contains(token), "shell does not apply {token}");
        }
    }

}
