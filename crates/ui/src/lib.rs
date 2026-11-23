//! Shared Slint UI for OpenWatch. Both the desktop and Android hosts build
//! against this library so the `.slint` documents are compiled exactly once.
//!
//! The generated window type never leaves this crate (Slint 1.19 issue #7060):
//! hosts call [`run_live`] with a [`DeviceLink`], or [`run_with`] with ordinary
//! Rust state.

slint::include_modules!();

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use openwatch_core::store::Store;
use slint::ComponentHandle;

pub mod ai;
pub mod bridge;
pub mod data_surface;
pub mod data_view;
pub mod link;

pub use bridge::{Action, AppError, AppState, DeviceLink, Reading, UiOptions};
use data_surface::DataView;
use data_view::WorkbenchData;
pub use link::{connect_watch, ConnectionLink};

/// Build, populate and run the shell with supplied state and options.
pub fn run_with(state: AppState, options: UiOptions) -> Result<(), AppError> {
    run_with_opts(state, options, false)
}

/// `refresh_on_start` schedules the first read for after the event loop starts
/// instead of blocking before the first paint.
fn run_with_opts(
    state: AppState,
    options: UiOptions,
    refresh_on_start: bool,
) -> Result<(), AppError> {
    let window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(state));
    state.borrow().apply_to(&window, &options);

    let workbench = Rc::new(RefCell::new(WorkbenchData::default()));
    window.set_wb_endpoint(slint::SharedString::from(workbench.borrow().endpoint()));
    {
        let mut wb = workbench.borrow_mut();
        wb.set_frames(state.borrow().stored_frames(200).unwrap_or_default());
        wb.apply_to(&window);
    }

    let sync_options = options;
    let sync_in_flight = Rc::new(Cell::new(false));
    let schedule_refresh: Rc<dyn Fn()> = Rc::new({
        let shared = state.clone();
        let view = window.clone_strong();
        let in_flight = sync_in_flight.clone();
        let wb = workbench.clone();
        move || {
            if in_flight.get() {
                return;
            }
            in_flight.set(true);
            view.set_syncing(true);
            let shared = shared.clone();
            let view = view.clone_strong();
            let flag = in_flight.clone();
            let wb = wb.clone();
            // The one-shot timer fires after this handler returns, so the
            // disabled/"Syncing…" state gets painted before the blocking read.
            // shortcut: refresh still runs on Slint's UI thread (~7 s) because
            // the window handle is !Send and AppState owns a !Send DeviceLink;
            // this is deferral, not a non-blocking refresh. Upgrade when the
            // link is Send + Sync and results can be marshalled by a timer.
            slint::Timer::single_shot(Duration::ZERO, move || {
                {
                    let mut state = shared.borrow_mut();
                    let _ = state.refresh_reads();
                    state.sync_connection();
                    state.apply_to(&view, &sync_options);
                }
                // The refresh logged new frames to the store: reload the raw
                // inspector so it reads what was actually sent and received.
                let frames = shared.borrow().stored_frames(200).unwrap_or_default();
                wb.borrow_mut().set_frames(frames);
                wb.borrow().apply_to(&view);
                view.set_syncing(false);
                flag.set(false);
            });
        }
    });
    window.on_sync_now({
        let schedule = schedule_refresh.clone();
        move || schedule()
    });

    let data = Rc::new(RefCell::new(DataView::default()));
    {
        let mut view = data.borrow_mut();
        view.load_units(&state.borrow());
        let _ = view.reload(&state.borrow());
        view.apply_to(&window);
    }

    let shared = state.clone();
    let view = window.clone_strong();
    window.on_reorder_card(move |key, delta| {
        let key = key.to_string();
        let mut state = shared.borrow_mut();
        state.move_card(&key, delta);
        state.apply_cards(&view);
    });

    // Today's Control is step one; only the sheet's Confirm step writes.
    {
        let app = state.clone();
        let view = window.clone_strong();
        window.on_open_control_sheet(move |index| {
            let mut app = app.borrow_mut();
            app.open_control_sheet(index as usize);
            app.apply_control_sheet(&view);
        });
    }
    {
        let app = state.clone();
        let view = window.clone_strong();
        window.on_close_control_sheet(move || {
            let mut app = app.borrow_mut();
            app.close_control_sheet();
            app.apply_control_sheet(&view);
        });
    }
    {
        let app = state.clone();
        let view = window.clone_strong();
        window.on_confirm_control_action(move || {
            let mut app = app.borrow_mut();
            let _ = app.confirm_control_action();
            app.apply_cards(&view);
            app.apply_control_sheet(&view);
        });
    }

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
            let prepared = {
                let mut wb = shared.borrow_mut();
                wb.set_note(view.get_wb_note().to_string());
                wb.set_endpoint(view.get_wb_endpoint().to_string());
                wb.set_key(view.get_wb_key().to_string());
                // No key means off: no transport is constructed or called.
                let prepared = wb.prepare_explain();
                if prepared.is_none() {
                    if !ai::enabled(&wb.config()) {
                        wb.set_explanation(Err(ai::AiError::Disabled));
                    } else {
                        wb.set_explanation(Err(ai::AiError::Response(
                            "select a frame first".into(),
                        )));
                    }
                }
                wb.apply_to(&view);
                prepared
            };
            let Some((config, hex, note)) = prepared else {
                return;
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

    {
        let shared = data.clone();
        let view = window.clone_strong();
        window.on_select_data_series(move |index| {
            let mut data = shared.borrow_mut();
            data.select(index);
            data.apply_to(&view);
        });
    }
    {
        let shared = data.clone();
        let app = state.clone();
        let view = window.clone_strong();
        window.on_export_json(move || {
            let mut data = shared.borrow_mut();
            data.export_json(&app.borrow());
            data.apply_to(&view);
        });
    }
    {
        let shared = data.clone();
        let app = state.clone();
        let view = window.clone_strong();
        window.on_export_csv(move || {
            let mut data = shared.borrow_mut();
            data.export_csv(&app.borrow());
            data.apply_to(&view);
        });
    }
    {
        let shared = data.clone();
        let view = window.clone_strong();
        window.on_request_delete(move || {
            let mut data = shared.borrow_mut();
            data.request_delete();
            data.apply_to(&view);
        });
    }
    {
        let shared = data.clone();
        let view = window.clone_strong();
        window.on_cancel_delete(move || {
            let mut data = shared.borrow_mut();
            data.cancel_delete();
            data.apply_to(&view);
        });
    }
    {
        let shared = data.clone();
        let app = state.clone();
        let view = window.clone_strong();
        window.on_confirm_delete(move || {
            let result = shared.borrow_mut().delete_all(&mut app.borrow_mut());
            if let Err(error) = result {
                shared.borrow_mut().set_status(error.to_string());
            }
            shared.borrow_mut().apply_to(&view);
        });
    }
    {
        let shared = data.clone();
        let app = state.clone();
        let view = window.clone_strong();
        window.on_toggle_units(move || {
            let result = shared.borrow_mut().toggle_units(&mut app.borrow_mut());
            if let Err(error) = result {
                shared.borrow_mut().set_status(error.to_string());
            }
            shared.borrow_mut().apply_to(&view);
        });
    }

    if refresh_on_start {
        schedule_refresh();
    }

    window.run()?;
    Ok(())
}

/// Build host state with the local store attached. One path for every host, so
/// the Data surface and the raw inspector always have persistence behind them.
fn live_state(link: impl DeviceLink + 'static, store: Store, now: fn() -> i64) -> AppState {
    let connection = link.connection();
    let mut state = AppState::with_link(link, now).with_store(store);
    state.connection = connection;
    state
}

/// Run the shell against a live watch connection and a local SQLite store.
///
/// The link's connection state is applied to the status strip; the first read
/// and every later "Sync now" use the same deferred, guarded refresh path, so
/// neither blocks before the first paint nor stacks overlapping refreshes.
pub fn run_live(
    link: impl DeviceLink + 'static,
    store: Store,
    options: UiOptions,
) -> Result<(), AppError> {
    run_with_opts(live_state(link, store, bridge::system_now), options, true)
}

/// Run the shell with default state and options.
pub fn run() -> Result<(), AppError> {
    run_with(AppState::default(), UiOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    #[derive(Debug)]
    struct OfflineLink;

    impl DeviceLink for OfflineLink {
        fn query(
            &self,
            _cmd: u8,
            _payload: &[u8],
            _wait_ms: u64,
        ) -> Result<openwatch_core::protocol::frame::Frame, AppError> {
            Err(AppError::NoLink)
        }
        fn write(&self, _frame: &[u8]) -> Result<(), AppError> {
            Err(AppError::NoLink)
        }
        fn battery(&self) -> Option<u8> {
            None
        }
    }

    #[test]
    fn live_state_always_carries_the_host_store() {
        let state = live_state(
            OfflineLink,
            Store::open(":memory:").unwrap(),
            bridge::system_now,
        );
        assert!(state.has_store(), "a host must never run store-less");
    }

    #[test]
    fn active_tab_label_contrast_meets_design_spec() {
        let window = include_str!("../ui/app-window.slint");
        assert!(
            window.contains("root.active-tab == i ? Theme.zinc-950 : Theme.text-primary"),
            "active tab must use the near-black label on the accent fill"
        );
        let theme = include_str!("../ui/theme.slint");
        assert!(theme.contains("accent: #0a84ff"));
        assert!(theme.contains("zinc-950: #09090b"));
        // WCAG 2.1 relative luminance and contrast ratio.
        fn luminance(hex: &str) -> f64 {
            let channel = |at: usize| {
                let value =
                    u8::from_str_radix(&hex[1 + at * 2..3 + at * 2], 16).unwrap() as f64 / 255.0;
                if value <= 0.03928 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2)
        }
        let accent = luminance("#0a84ff");
        let label = luminance("#09090b");
        let ratio = (accent + 0.05) / (label + 0.05);
        assert!(
            ratio >= 4.5,
            "active tab contrast is {ratio:.2}:1, design §6 requires ≥4.5:1"
        );
    }

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
            "Mark reviewed",
            "preview only",
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
            include_str!("../ui/data.slint"),
        ] {
            assert!(
                !surface.contains("frame-hex"),
                "hex leaked into a human surface"
            );
            assert!(
                !surface.contains("Advanced actions"),
                "unverified actions leaked into a human surface"
            );
        }
    }

    #[test]
    fn data_surface_covers_history_export_delete_and_units() {
        let data = include_str!("../ui/data.slint");
        for token in [
            "History",
            "Export JSON",
            "Export CSV",
            "Delete all local data",
            "Delete every reading and frame",
            "Units:",
            "Confirm",
            "Cancel",
        ] {
            assert!(data.contains(token), "data surface lost {token}");
        }
        let shell = include_str!("../ui/app-window.slint");
        assert!(shell.contains("DataPane"), "shell lost the Data pane");
        assert!(
            shell.contains("data-series"),
            "shell lost Data history wiring"
        );
    }

    #[test]
    fn today_control_two_step_confirm_writes_exact_goal_frame() {
        use std::cell::RefCell;
        use std::rc::Rc;

        use openwatch_core::protocol::frame::Frame;

        #[derive(Debug)]
        struct RecordingLink(Rc<RefCell<Vec<Vec<u8>>>>);

        impl DeviceLink for RecordingLink {
            fn query(&self, _cmd: u8, _payload: &[u8], _wait_ms: u64) -> Result<Frame, AppError> {
                Err(AppError::NoLink)
            }
            fn write(&self, frame: &[u8]) -> Result<(), AppError> {
                self.0.borrow_mut().push(frame.to_vec());
                Ok(())
            }
            fn battery(&self) -> Option<u8> {
                None
            }
        }

        let writes = Rc::new(RefCell::new(Vec::new()));
        let mut state = AppState::with_link(RecordingLink(writes.clone()), bridge::system_now);
        state.cards.push(Reading {
            key: "goals-steps",
            group: "Today",
            title: "Step goal",
            value: "8000".into(),
            detail: "Daily goal stored on the watch",
            updated_at: "2026-11-22 14:03 UTC".into(),
        });

        // First click arms the control and opens the sheet; it must not write.
        assert!(
            state.open_control_sheet(0),
            "a goal card must expose a control"
        );
        assert!(state.control_sheet_open(), "sheet must open on first click");
        assert!(writes.borrow().is_empty(), "arming a control wrote a frame");

        // Second click confirms the armed action and writes the exact frame.
        state.confirm_control_action().unwrap();
        assert!(
            !state.control_sheet_open(),
            "sheet must close after confirm"
        );
        let recorded = writes.borrow();
        assert_eq!(recorded.len(), 1, "confirm must write exactly one frame");
        let hex: String = recorded[0]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(hex, "feea20091600001f40");
    }

    #[test]
    fn only_l_verified_cards_advertise_a_control() {
        assert!(AppState::card_writable("goals-steps"));
        assert!(AppState::card_writable("heart-rate"));
        for key in ["battery", "steps", "stress", "move-reminders"] {
            assert!(!AppState::card_writable(key), "{key} has no write mapping");
        }
        let today = include_str!("../ui/today.slint");
        assert!(
            today.contains("if card.writable"),
            "control must be gated on the write mapping"
        );
        assert!(
            !today.contains("☰ Control"),
            "inert control decoy is still present"
        );
        let shell = include_str!("../ui/app-window.slint");
        for token in [
            "control-sheet-open",
            "open-control-sheet",
            "close-control-sheet",
            "confirm-control-action",
        ] {
            assert!(shell.contains(token), "shell lost {token}");
        }
    }
}
