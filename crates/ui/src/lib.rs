//! Shared Slint UI for OpenWatch. Both the desktop and Android hosts build
//! against this library so the `.slint` documents are compiled exactly once.
//!
//! The generated window type never leaves this crate (Slint 1.19 issue #7060):
//! hosts call [`run`] or [`run_with`] with ordinary Rust state instead.

slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

pub mod ai;
pub mod bridge;
pub mod data_view;

pub use bridge::{Action, AppError, AppState, DeviceLink, Reading, UiOptions};
use data_view::WorkbenchData;

/// Build, populate and run the shell with supplied state and options.
pub fn run_with(state: AppState, options: UiOptions) -> Result<(), AppError> {
    let window = AppWindow::new()?;
    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    state.borrow().apply_to(&window, &options);

    let workbench = Rc::new(RefCell::new(WorkbenchData::default()));
    window.set_wb_endpoint(slint::SharedString::from(workbench.borrow().endpoint()));
    {
        let mut wb = workbench.borrow_mut();
        wb.set_frames(state.borrow().stored_frames(200).unwrap_or_default());
        wb.apply_to(&window);
    }

    let shared = state.clone();
    let view = window.clone_strong();
    window.on_reorder_card(move |key, delta| {
        let key = key.to_string();
        let mut state = shared.borrow_mut();
        state.move_card(&key, delta);
        state.apply_cards(&view);
    });

    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_select_frame(move |id| {
            let mut wb = shared.borrow_mut();
            wb.select(id);
            wb.apply_to(&view);
        });
    }

    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_key_edited(move |key| {
            let mut wb = shared.borrow_mut();
            wb.set_key(key.to_string());
            wb.apply_to(&view);
        });
    }

    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_preview_payload(move || {
            let mut wb = shared.borrow_mut();
            wb.set_note(view.get_wb_note().to_string());
            wb.preview();
            wb.apply_to(&view);
        });
    }

    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_explain_frame(move || {
            let (config, hex, note) = {
                let mut wb = shared.borrow_mut();
                wb.set_note(view.get_wb_note().to_string());
                wb.set_endpoint(view.get_wb_endpoint().to_string());
                wb.set_key(view.get_wb_key().to_string());
                // No key means off: no transport is constructed or called.
                if !ai::enabled(&wb.config()) {
                    wb.set_explanation(Err(ai::AiError::Disabled));
                    wb.apply_to(&view);
                    return;
                }
                let Some(hex) = wb.selected_hex().map(str::to_string) else {
                    wb.set_explanation(Err(ai::AiError::Response("select a frame first".into())));
                    wb.apply_to(&view);
                    return;
                };
                let config = wb.config();
                wb.mark_sending();
                wb.apply_to(&view);
                (config, hex, wb.note().to_string())
            };
            // shortcut: synchronous POST keeps the generated window handle off
            // other threads (it is !Send); move to a channel + timer if a call
            // ever takes long enough to block the UI.
            let result = ai::explain_blocking_with_note(&config, &hex, &note);
            let mut wb = shared.borrow_mut();
            wb.set_explanation(result);
            wb.apply_to(&view);
        });
    }

    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_request_action(move |key| {
            let mut wb = shared.borrow_mut();
            wb.request_action(&key);
            wb.apply_to(&view);
        });
    }
    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_confirm_action(move || {
            let mut wb = shared.borrow_mut();
            wb.confirm_action();
            wb.apply_to(&view);
        });
    }
    {
        let shared = workbench.clone();
        let view = window.clone_strong();
        window.on_cancel_action(move || {
            let mut wb = shared.borrow_mut();
            wb.cancel_action();
            wb.apply_to(&view);
        });
    }

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
        assert_eq!(
            set("connected"),
            slint::Color::from_rgb_u8(0x30, 0xD1, 0x58)
        );
        for state in ["reconnecting", "scanning", "connecting"] {
            assert_eq!(
                set(state),
                slint::Color::from_rgb_u8(0xFF, 0x9F, 0x0A),
                "{state}"
            );
        }
        assert_eq!(
            set("not found"),
            slint::Color::from_rgb_u8(0xFF, 0x45, 0x3A)
        );
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

    #[test]
    fn device_grid_rows_fit_and_do_not_overlap() {
        let device = include_str!("../ui/device.slint");
        for expression in [
            "Math.mod(i, 3) * (parent.width / 3) + Theme.space-3",
            "Math.floor(i / 3) * 88px",
            "width: parent.width / 3 - Theme.space-6",
            "height: 84px",
        ] {
            assert!(device.contains(expression), "device grid lost {expression}");
        }
        let columns = 3i32;
        let rows = 15i32;
        let stride = 88i32;
        let height = 84i32;
        let bands = (rows + columns - 1) / columns;
        let last_bottom = (bands - 1) * stride + height;
        assert!(
            last_bottom <= 560,
            "last device row ends at {last_bottom}px, past the ~560px content area"
        );
        // Column c: x = c*step + space-3, width = step - space-6; the next
        // column starts at (c+1)*step + space-3, so the gap must stay positive.
        let step = 792 / columns; // AppWindow content width / 3
        let first_right = 12 + (step - 24);
        let next_left = step + 12;
        assert!(first_right < next_left, "same-band columns overlap");
    }

    #[test]
    fn workbench_is_the_only_hex_surface() {
        let workbench = include_str!("../ui/workbench.slint");
        for token in [
            "Frame hex",
            "Source frame:",
            "Exact payload (review before send)",
            "S source / U unverified",
            "Confirm",
            "key-edited",
        ] {
            assert!(workbench.contains(token), "workbench lost {token}");
        }
        let shell = include_str!("../ui/app-window.slint");
        assert!(
            shell.contains("key-edited"),
            "key editing is not wired to the host"
        );
        for surface in [
            include_str!("../ui/today.slint"),
            include_str!("../ui/device.slint"),
        ] {
            assert!(
                !surface.contains("frame-hex"),
                "hex leaked into a human surface"
            );
        }
    }
}
