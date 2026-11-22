//! Read-only view models for the Workbench raw inspector.
//!
//! This is the one surface that renders raw hex. Today and Device consume only
//! decoded readings; nothing here writes back to the device.

use openwatch_core::protocol::frame;
use openwatch_core::protocol::registry::{self, Confidence};
use openwatch_core::store::FrameRow;
use slint::{ModelRc, SharedString, VecModel};

use crate::ai::{payload, AiConfig, AiError};
use crate::bridge::utc_label;
use crate::{ActionView, AppWindow, FrameView};

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:8000/v1/chat/completions";

/// An advanced action listed with its evidence tag. Running one is a separate,
/// explicitly confirmed step; this panel never auto-executes it.
#[derive(Debug, Clone)]
pub struct AdvancedAction {
    pub key: String,
    pub title: String,
    pub tag: &'static str,
    pub detail: String,
}

/// All source/unverified capabilities, tagged S or U.
pub fn advanced_actions() -> Vec<AdvancedAction> {
    registry::all_opcodes()
        .into_iter()
        .filter(|(_, _, confidence)| {
            matches!(confidence, Confidence::Source | Confidence::Unverified)
        })
        .map(|(cmd, name, confidence)| {
            let source = confidence == Confidence::Source;
            AdvancedAction {
                key: format!("{cmd:02X}"),
                title: name.to_string(),
                tag: if source { "S" } else { "U" },
                detail: format!(
                    "0x{cmd:02X} · {}",
                    if source {
                        "source-verified"
                    } else {
                        "unverified"
                    }
                ),
            }
        })
        .collect()
}

/// Evidence tag and long label for a stored confidence string.
pub fn confidence_tag(level: &str) -> (&'static str, &'static str) {
    match level {
        "live" => ("L", "LIVE"),
        "source" => ("S", "SOURCE"),
        _ => ("U", "UNVERIFIED"),
    }
}

fn hex_bytes(hex: &str) -> Vec<u8> {
    let compact: String = hex
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if compact.len() % 2 != 0 {
        return Vec::new();
    }
    (0..compact.len())
        .step_by(2)
        .filter_map(|index| u8::from_str_radix(&compact[index..index + 2], 16).ok())
        .collect()
}

/// Command byte and registered name for a stored frame, best-effort.
pub fn opcode_for(hex: &str) -> (String, String) {
    let bytes = hex_bytes(hex);
    let Ok(parsed) = frame::parse(&bytes) else {
        return ("—".into(), "unparsed".into());
    };
    let sub = parsed.payload.first().copied();
    let name = registry::all_opcodes()
        .into_iter()
        .find(|(cmd, name, _)| {
            *cmd == parsed.cmd
                && if parsed.cmd == 0xB9 {
                    sub.map_or(true, |value| name.contains(&format!("B9 {value:02X}")))
                } else {
                    true
                }
        })
        .map(|(_, name, _)| name.to_string())
        .unwrap_or_else(|| "unknown opcode".into());
    (format!("0x{:02X}", parsed.cmd), name)
}

fn frame_view(row: &FrameRow) -> FrameView {
    let (cmd, opcode) = opcode_for(&row.frame_hex);
    let (tag, confidence) = confidence_tag(&row.confidence);
    FrameView {
        id: row.id as i32,
        time: SharedString::from(utc_label(row.at_unix).as_str()),
        direction: SharedString::from(row.direction.as_str()),
        cmd: SharedString::from(cmd.as_str()),
        opcode: SharedString::from(opcode.as_str()),
        confidence: SharedString::from(confidence),
        confidence_tag: SharedString::from(tag),
        hex: SharedString::from(row.frame_hex.as_str()),
    }
}

fn action_view(action: &AdvancedAction) -> ActionView {
    ActionView {
        key: SharedString::from(action.key.as_str()),
        title: SharedString::from(action.title.as_str()),
        tag: SharedString::from(action.tag),
        detail: SharedString::from(action.detail.as_str()),
    }
}

/// State behind the Workbench surface.
pub struct WorkbenchData {
    frames: Vec<FrameRow>,
    actions: Vec<AdvancedAction>,
    selected: Option<usize>,
    note: String,
    endpoint: String,
    key: Option<String>,
    preview: String,
    explanation: String,
    status: String,
    pending: Option<usize>,
    confirmed: Vec<String>,
}

impl Default for WorkbenchData {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            actions: advanced_actions(),
            selected: None,
            note: String::new(),
            endpoint: DEFAULT_ENDPOINT.into(),
            key: None,
            preview: String::new(),
            explanation: String::new(),
            status: "AI off — add a key to enable".into(),
            pending: None,
            confirmed: Vec::new(),
        }
    }
}

impl WorkbenchData {
    /// Replace the table contents, keeping a selection only if it still exists.
    pub fn set_frames(&mut self, frames: Vec<FrameRow>) {
        self.selected = self.selected.and_then(|index| {
            let id = self.frames.get(index).map(|row| row.id);
            frames.iter().position(|row| Some(row.id) == id)
        });
        self.frames = frames;
    }

    pub fn config(&self) -> AiConfig {
        AiConfig {
            endpoint: self.endpoint.clone(),
            key: self.key.clone(),
        }
    }

    /// The default endpoint, applied once at startup; afterwards the field in
    /// the UI owns its text so redraws cannot wipe what the user typed.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn selected_hex(&self) -> Option<&str> {
        self.selected
            .and_then(|index| self.frames.get(index))
            .map(|row| row.frame_hex.as_str())
    }

    pub fn select(&mut self, id: i32) {
        self.selected = self.frames.iter().position(|row| row.id == id as i64);
        self.explanation.clear();
    }

    pub fn set_note(&mut self, note: String) {
        self.note = note;
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        self.endpoint = endpoint;
    }

    pub fn set_key(&mut self, key: String) {
        self.key = (!key.trim().is_empty()).then_some(key);
        self.status = if self.key.is_some() {
            "AI on — review the payload before sending".into()
        } else {
            "AI off — add a key to enable".into()
        };
    }

    /// Build the exact body that would be sent. Nothing is transmitted here.
    pub fn preview(&mut self) {
        let Some(hex) = self.selected_hex().map(str::to_string) else {
            self.status = "Select a frame first".into();
            return;
        };
        self.preview = payload(&hex, &self.note);
        self.status = "Payload ready — nothing sent yet".into();
    }

    /// Show that one frame is on its way to the configured endpoint.
    pub fn mark_sending(&mut self) {
        self.status = "Sending the selected frame…".into();
    }

    /// Record the result of an interpretation performed by the host.
    pub fn set_explanation(&mut self, result: Result<String, AiError>) {
        match result {
            Ok(text) => {
                self.explanation = text;
                self.status = "Explained".into();
            }
            Err(error) => {
                self.explanation.clear();
                self.status = error.to_string();
            }
        }
    }

    /// First step of an advanced action: show what would run, do not run it.
    pub fn request_action(&mut self, key: &str) {
        self.pending = self.actions.iter().position(|action| action.key == key);
        if let Some(index) = self.pending {
            self.status = format!("Confirm {} before running", self.actions[index].title);
        }
    }

    /// Second, explicit step: record the confirmed action locally.
    pub fn confirm_action(&mut self) {
        if let Some(index) = self.pending.take() {
            let action = &self.actions[index];
            self.confirmed
                .push(format!("[{}] {}", action.tag, action.title));
            self.status = format!("Confirmed: {}", action.title);
        }
    }

    pub fn cancel_action(&mut self) {
        self.pending = None;
        self.status = "Action cancelled".into();
    }

    /// Copy the current state into the generated window (this crate only).
    pub fn apply_to(&self, window: &AppWindow) {
        let frames: Vec<FrameView> = self.frames.iter().map(frame_view).collect();
        window.set_frames(ModelRc::new(VecModel::from(frames)));
        let actions: Vec<ActionView> = self.actions.iter().map(action_view).collect();
        window.set_actions(ModelRc::new(VecModel::from(actions)));
        window.set_selected_frame(
            self.selected
                .and_then(|index| self.frames.get(index))
                .map_or(-1, |row| row.id as i32),
        );

        let selected = self
            .selected
            .and_then(|index| self.frames.get(index))
            .map(frame_view);
        let field = |value: &str| SharedString::from(value);
        window.set_selected_time(field(selected.as_ref().map_or("", |row| row.time.as_str())));
        window.set_selected_direction(field(
            selected.as_ref().map_or("", |row| row.direction.as_str()),
        ));
        window.set_selected_cmd(field(selected.as_ref().map_or("", |row| row.cmd.as_str())));
        window.set_selected_opcode(field(
            selected.as_ref().map_or("", |row| row.opcode.as_str()),
        ));
        window.set_selected_confidence(field(
            selected.as_ref().map_or("", |row| row.confidence.as_str()),
        ));
        window.set_selected_hex(field(selected.as_ref().map_or("", |row| row.hex.as_str())));

        // note / endpoint / key are owned by the TextInputs (two-way bound);
        // writing them here would erase typing on every redraw.
        window.set_wb_has_key(self.key.is_some());
        window.set_wb_preview(field(&self.preview));
        window.set_wb_explanation(field(&self.explanation));
        window.set_wb_status(field(&self.status));
        window.set_wb_pending(field(
            self.pending
                .and_then(|index| self.actions.get(index))
                .map_or("", |action| action.title.as_str()),
        ));
        window.set_wb_confirmed(field(&self.confirmed.join("\n")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, hex: &str, confidence: &str) -> FrameRow {
        FrameRow {
            id,
            direction: "rx".into(),
            frame_hex: hex.into(),
            at_unix: 1_795_356_180,
            confidence: confidence.into(),
        }
    }

    #[test]
    fn frame_view_shows_time_direction_cmd_confidence_and_hex() {
        let view = frame_view(&row(3, "FEEA200553", "live"));
        assert_eq!(view.time.as_str(), "2026-11-22 14:03 UTC");
        assert_eq!(view.direction.as_str(), "rx");
        assert_eq!(view.cmd.as_str(), "0x53");
        assert_eq!(view.confidence.as_str(), "LIVE");
        assert_eq!(view.confidence_tag.as_str(), "L");
        assert_eq!(view.hex.as_str(), "FEEA200553");
    }

    #[test]
    fn opcode_lookup_uses_registered_names_and_b9_subcommands() {
        assert_eq!(opcode_for("FEEA20091600001F40").1, "set step goal");
        let (_, name) = opcode_for("FEEA2007B91504");
        assert!(name.contains("alarms"), "{name}");
        assert_eq!(opcode_for("garbage").0, "—");
    }

    #[test]
    fn advanced_actions_are_tagged_s_or_u() {
        let actions = advanced_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().all(|action| matches!(action.tag, "S" | "U")));
        assert!(actions.iter().any(|action| action.tag == "S"));
        assert!(actions.iter().any(|action| action.tag == "U"));
    }

    #[test]
    fn advanced_action_needs_two_steps() {
        let mut data = WorkbenchData::default();
        let key = advanced_actions()[0].key.clone();
        data.request_action(&key);
        assert!(
            data.confirmed.is_empty(),
            "a click alone must not run an action"
        );
        assert!(data.pending.is_some());
        data.confirm_action();
        assert_eq!(data.confirmed.len(), 1);
    }
}
