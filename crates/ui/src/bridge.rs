//! Rust-side state that feeds the generated Slint window.
//!
//! The generated window type stays inside this crate: Slint 1.19 cannot export
//! it across crate boundaries. Hosts drive the UI through [`crate::run`].
//!
//! Today and Device show decoded, human-readable values only. Raw frames stay
//! in the Workbench (Task 10).

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use openwatch_core::ble::BleError;
use openwatch_core::connection::ConnectionState;
use openwatch_core::protocol::command;
use openwatch_core::protocol::decode::{decode, decode_message_apps, Value};
use openwatch_core::protocol::frame::{packet, Frame};
use openwatch_core::protocol::registry;
use openwatch_core::store::{FrameRow, Store};
use slint::{ModelRc, SharedString, VecModel};
use thiserror::Error;

use crate::AppWindow;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("UI error: {0}")]
    Ui(#[from] slint::PlatformError),
    #[error("device error: {0}")]
    Device(#[from] BleError),
    #[error("store error: {0}")]
    Store(#[from] openwatch_core::store::StoreError),
    #[error("async runtime error: {0}")]
    Runtime(#[from] std::io::Error),
    #[error("no device connection configured")]
    NoLink,
}

/// The connection-manager write/query path, reduced to what the UI needs.
/// The desktop/Android hosts supply a real connection; tests use an offline fake.
pub trait DeviceLink: fmt::Debug {
    fn query(&self, cmd: u8, payload: &[u8], wait_ms: u64) -> Result<Frame, AppError>;
    fn write(&self, frame: &[u8]) -> Result<(), AppError>;
    fn battery(&self) -> Option<u8>;

    /// Current connection state, for the shell's status strip. Links that do
    /// not track a connection report `Idle`.
    fn connection(&self) -> ConnectionState {
        ConnectionState::Idle
    }
}

/// Write-side actions exposed by the Control affordances and by hosts.
#[derive(Debug, Clone)]
pub enum Action {
    SetGoal(u32),
    SetTimeSystem {
        h24: bool,
    },
    SetDnd {
        start: (u8, u8),
        end: (u8, u8),
        enable: bool,
    },
    SetAlarm {
        slot: u8,
        hour: u8,
        minute: u8,
        repeat: u8,
        date: (u16, u8, u8),
    },
    DeleteAlarm(u8),
    Weather {
        cond: u8,
        temp: i8,
        city: String,
        festival: String,
    },
    Message(String),
    TimeSync(u32),
    StartHr,
}

/// One human-readable value plus the UTC instant it was last updated.
#[derive(Debug, Clone)]
pub struct Reading {
    pub key: &'static str,
    pub group: &'static str,
    pub title: &'static str,
    pub value: String,
    pub detail: &'static str,
    pub updated_at: String,
}

fn reading(
    key: &'static str,
    group: &'static str,
    title: &'static str,
    value: impl Into<String>,
    detail: &'static str,
    stamp: &str,
) -> Reading {
    Reading {
        key,
        group,
        title,
        value: value.into(),
        detail,
        updated_at: stamp.to_string(),
    }
}

/// State shown in the shell and on the Today / Device surfaces.
pub struct AppState {
    pub device_name: String,
    pub battery: u8,
    pub connection: ConnectionState,
    /// Pre-formatted UTC label, e.g. `2026-11-22 14:03 UTC`.
    pub last_sync: Option<String>,
    /// UI-owned flags. The core connection stays authoritative for "connected";
    /// these describe the last refresh, which can fail while still connected.
    pub syncing: bool,
    pub failed: bool,
    pub last_error: Option<String>,
    /// Epoch seconds of the last successful refresh, for the stale threshold.
    last_refresh_ok: Option<i64>,
    /// Today's reorderable metric cards.
    pub cards: Vec<Reading>,
    /// Device identity, settings, apps, alarms, goals and faces.
    pub device_values: Vec<Reading>,
    /// Two-step control sheet: the first click arms, the second writes.
    control_sheet_open: bool,
    armed_action: Option<Action>,
    link: Option<Box<dyn DeviceLink>>,
    store: Option<Store>,
    now: fn() -> i64,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("device_name", &self.device_name)
            .field("battery", &self.battery)
            .field("connection", &self.connection)
            .field("last_sync", &self.last_sync)
            .field("syncing", &self.syncing)
            .field("failed", &self.failed)
            .field("last_error", &self.last_error)
            .field("cards", &self.cards)
            .field("device_values", &self.device_values)
            .finish_non_exhaustive()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            device_name: "Fire-Boltt 039".into(),
            battery: 0,
            connection: ConnectionState::Idle,
            last_sync: None,
            syncing: false,
            failed: false,
            last_error: None,
            last_refresh_ok: None,
            cards: Vec::new(),
            device_values: Vec::new(),
            control_sheet_open: false,
            armed_action: None,
            link: None,
            store: None,
            now: system_now,
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
        ConnectionState::Idle => "Idle",
        ConnectionState::Scanning => "Scanning",
        ConnectionState::Connecting => "Connecting",
        ConnectionState::Connected => "Connected",
        ConnectionState::Reconnecting => "Reconnecting",
        ConnectionState::NotFound => "Not found",
    }
}

fn last_sync_label(value: Option<&str>) -> String {
    match value {
        Some(stamp) => format!("Synced {stamp}"),
        None => "Never synced".into(),
    }
}

/// A refresh older than this is stale, even while the link reports connected.
const STALE_AFTER_SECS: i64 = 5 * 60;

/// The displayed sync state. Connection state still comes from the core; this
/// adds the UI-owned refresh result on top of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPhase {
    Syncing,
    Failed,
    Stale,
    Connected,
    Idle,
    Scanning,
    Connecting,
    Reconnecting,
    NotFound,
}

impl SyncPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Syncing => "Syncing",
            Self::Failed => "Sync failed — retry",
            Self::Stale => "Stale · sync now",
            Self::Connected => "Connected",
            Self::Idle => "Idle",
            Self::Scanning => "Scanning",
            Self::Connecting => "Connecting",
            Self::Reconnecting => "Reconnecting",
            Self::NotFound => "Not found",
        }
    }
}

pub(crate) fn system_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Format UTC epoch seconds without pulling a date-time crate into the UI.
pub(crate) fn utc_label(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let seconds = unix.rem_euclid(86_400);
    let leap = |year: i64| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let mut year = 1970;
    let mut remaining = days;
    loop {
        let length = if leap(year) { 366 } else { 365 };
        if remaining < length {
            break;
        }
        remaining -= length;
        year += 1;
    }
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while remaining >= lengths[month] {
        remaining -= lengths[month];
        month += 1;
    }
    format!(
        "{year:04}-{:02}-{:02} {:02}:{:02} UTC",
        month + 1,
        remaining + 1,
        seconds / 3600,
        (seconds % 3600) / 60
    )
}

fn hhmm(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Goals reply: [0x10][?] then [type u8][LE u32] pairs from byte 2.
fn parse_goals(bytes: &[u8]) -> Vec<(u8, u32)> {
    bytes
        .get(2..)
        .unwrap_or_default()
        .as_chunks::<5>()
        .0
        .iter()
        .map(|chunk| {
            (
                chunk[0],
                u32::from_le_bytes([chunk[1], chunk[2], chunk[3], chunk[4]]),
            )
        })
        .collect()
}

/// Reply window for one read query. Kept short because a full refresh issues
/// ~13 queries; the write path still enforces the protocol's 350 ms spacing.
pub const REFRESH_WAIT_MS: u64 = 150;

/// Uppercase hex of the exact bytes put on (or received from) the wire.
fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

/// L-verified write mappings for human-readable readings. A reading with no
/// mapping must not advertise a Control affordance.
fn action_for(reading: &Reading) -> Option<Action> {
    match reading.key {
        "goals-steps" => Some(Action::SetGoal(reading.value.parse().ok()?)),
        "heart-rate" => Some(Action::StartHr),
        _ => None,
    }
}

fn action_label(action: &Action) -> String {
    match action {
        Action::SetGoal(steps) => format!("Set the daily step goal to {steps}."),
        Action::StartHr => "Start a live heart-rate reading on the watch.".to_string(),
        _ => "Write this setting to the watch.".to_string(),
    }
}

impl AppState {
    /// Attach a device connection and a clock used for update timestamps.
    pub fn with_link(link: impl DeviceLink + 'static, now: fn() -> i64) -> Self {
        Self {
            link: Some(Box::new(link)),
            now,
            ..Self::default()
        }
    }

    /// Persist every decoded reading to the local SQLite store.
    pub fn with_store(mut self, store: Store) -> Self {
        self.store = Some(store);
        self
    }

    /// True when a local store is attached. Production hosts build their state
    /// through `run_live`, which must always attach one.
    #[cfg(test)]
    pub(crate) fn has_store(&self) -> bool {
        self.store.is_some()
    }

    /// Refresh the status strip from the link; reconnects are not static.
    pub(crate) fn sync_connection(&mut self) {
        let current = self.link.as_ref().map(|link| link.connection());
        if let Some(current) = current {
            self.connection = current;
        }
    }

    /// An in-flight refresh wins, then a failed one, then the core connection
    /// (downgraded to stale when no refresh has succeeded in five minutes).
    pub fn sync_phase(&self) -> SyncPhase {
        if self.syncing {
            return SyncPhase::Syncing;
        }
        if self.failed {
            return SyncPhase::Failed;
        }
        match self.connection {
            ConnectionState::Connected => {
                let stale = self
                    .last_refresh_ok
                    .is_some_and(|at| (self.now)().saturating_sub(at) > STALE_AFTER_SECS);
                if stale {
                    SyncPhase::Stale
                } else {
                    SyncPhase::Connected
                }
            }
            ConnectionState::Idle => SyncPhase::Idle,
            ConnectionState::Scanning => SyncPhase::Scanning,
            ConnectionState::Connecting => SyncPhase::Connecting,
            ConnectionState::Reconnecting => SyncPhase::Reconnecting,
            ConnectionState::NotFound => SyncPhase::NotFound,
        }
    }

    /// Most-recent frames for the Workbench raw inspector. No store configured
    /// means no frames; the surface stays empty rather than inventing data.
    pub fn stored_frames(&self, limit: i64) -> Result<Vec<FrameRow>, AppError> {
        match &self.store {
            Some(store) => Ok(store.frames(limit)?),
            None => Ok(Vec::new()),
        }
    }

    /// Distinct reading keys in the local store, for the history selector.
    pub fn reading_keys(&self) -> Result<Vec<String>, AppError> {
        match &self.store {
            Some(store) => Ok(store.reading_keys()?),
            None => Ok(Vec::new()),
        }
    }

    /// Timestamped values for one reading key.
    pub fn readings(&self, key: &str) -> Result<Vec<(i64, String)>, AppError> {
        match &self.store {
            Some(store) => Ok(store.readings(key)?),
            None => Ok(Vec::new()),
        }
    }

    /// Serialize everything the store holds to JSON (empty string with no store).
    pub fn export_json(&self) -> Result<String, AppError> {
        match &self.store {
            Some(store) => Ok(store.export_json()?),
            None => Ok(String::new()),
        }
    }

    /// Serialize everything the store holds to CSV (empty string with no store).
    pub fn export_csv(&self) -> Result<String, AppError> {
        match &self.store {
            Some(store) => Ok(store.export_csv()?),
            None => Ok(String::new()),
        }
    }

    /// Erase all locally stored readings and frames.
    pub fn delete_all(&mut self) -> Result<(), AppError> {
        if let Some(store) = self.store.as_mut() {
            store.delete_all()?;
        }
        Ok(())
    }

    /// Persist the local display-units preference as a reading.
    pub fn set_units(&mut self, units: &str) -> Result<(), AppError> {
        if let Some(store) = self.store.as_mut() {
            store.put_reading("units", units, (self.now)())?;
        }
        Ok(())
    }

    /// Last saved display-units preference, if any.
    pub fn units(&self) -> Option<String> {
        self.store
            .as_ref()
            .and_then(|store| store.readings("units").ok())
            .and_then(|rows| rows.last().map(|(_, value)| value.clone()))
    }

    pub fn reading(&self, key: &str) -> Option<&Reading> {
        self.cards
            .iter()
            .chain(self.device_values.iter())
            .find(|reading| reading.key == key)
    }

    /// Move a card by `delta` positions, clamped to the list.
    pub fn move_card(&mut self, key: &str, delta: i32) {
        let Some(from) = self.cards.iter().position(|card| card.key == key) else {
            return;
        };
        let to = (from as i32 + delta).clamp(0, self.cards.len() as i32 - 1) as usize;
        let card = self.cards.remove(from);
        self.cards.insert(to, card);
    }

    /// Query the live read opcodes and rebuild both surfaces. A missing reply
    /// simply omits its value; one unavailable read never blanks the rest.
    /// A hard failure is recorded for the failure state instead of discarded.
    pub fn refresh_reads(&mut self) -> Result<(), AppError> {
        match self.try_refresh_reads() {
            Ok(()) => {
                self.failed = false;
                self.last_error = None;
                self.last_refresh_ok = Some((self.now)());
                Ok(())
            }
            Err(error) => {
                self.failed = true;
                self.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn try_refresh_reads(&mut self) -> Result<(), AppError> {
        let link: &dyn DeviceLink = self
            .link
            .as_ref()
            .map(|link| link.as_ref())
            .ok_or(AppError::NoLink)?;
        let at = (self.now)();
        let stamp = utc_label(at);
        let mut cards = Vec::new();
        let mut device_values = Vec::new();
        // (direction, exact frame hex, cmd) for the raw store. The cmd drives
        // the evidence tag; the Workbench decodes cmd/opcode from the hex.
        let mut frames: Vec<(&'static str, String, u8)> = Vec::new();
        // Log both directions at the link boundary: the request is sent even
        // when the reply times out, so it is queued before the query.
        let mut read = |link: &dyn DeviceLink, cmd: u8, payload: &[u8]| -> Option<Value> {
            frames.push(("tx", hex_of(&packet(cmd, payload)), cmd));
            let frame = link.query(cmd, payload, REFRESH_WAIT_MS).ok()?;
            frames.push(("rx", hex_of(&packet(frame.cmd, &frame.payload)), frame.cmd));
            decode(frame.cmd, &frame.payload)
        };

        let battery = link.battery();
        if let Some(value) = battery {
            cards.push(reading(
                "battery",
                "Today",
                "Battery",
                format!("{value}%"),
                "Watch battery level",
                &stamp,
            ));
        }
        if let Some(Value::Number(steps)) = read(link, 0x2B, &[]) {
            cards.push(reading(
                "steps",
                "Today",
                "Steps",
                steps.to_string(),
                "Steps recorded today",
                &stamp,
            ));
        }
        if let Some(Value::Bytes(hr)) = read(link, 0x81, &[]) {
            if let Some(bpm) = hr.first() {
                cards.push(reading(
                    "heart-rate",
                    "Today",
                    "Heart rate",
                    format!("{bpm} bpm"),
                    "Latest heart-rate sample",
                    &stamp,
                ));
            }
        }
        if let Some(Value::Bytes(stress)) = read(link, 0xB9, &[0x11]) {
            cards.push(reading(
                "stress",
                "Today",
                "Stress",
                stress.get(1).copied().unwrap_or(0).to_string(),
                "Latest stress index",
                &stamp,
            ));
        }
        if let Some(Value::Bytes(goals)) = read(link, 0xB9, &[0x10]) {
            for (kind, value) in parse_goals(&goals) {
                let (key, title) = match kind {
                    1 => ("goals-steps", "Step goal"),
                    2 => ("goal-2", "Goal 2"),
                    3 => ("goal-3", "Derived goal"),
                    _ => ("goal-4", "Goal 4"),
                };
                cards.push(reading(
                    key,
                    "Today",
                    title,
                    value.to_string(),
                    "Daily goal stored on the watch",
                    &stamp,
                ));
                device_values.push(reading(
                    key,
                    "Goals",
                    title,
                    value.to_string(),
                    "Daily goal stored on the watch",
                    &stamp,
                ));
            }
        }

        device_values.push(reading(
            "identity",
            "Identity",
            "Device",
            self.device_name.clone(),
            "Connected watch",
            &stamp,
        ));
        if let Some(Value::Text(firmware)) = read(link, 0xBA, &[]) {
            device_values.push(reading(
                "firmware",
                "Identity",
                "Installed firmware",
                firmware,
                "Version reported by the watch",
                &stamp,
            ));
        }
        if let Some(Value::Number(limit)) = read(link, 0xBB, &[]) {
            device_values.push(reading(
                "message-limit",
                "Messages",
                "Message length limit",
                limit.to_string(),
                "UTF-8 bytes per message",
                &stamp,
            ));
        }
        if let Some(Value::Text(system)) = read(link, 0x27, &[]) {
            device_values.push(reading(
                "time-system",
                "Settings",
                "Time format",
                system,
                "Watch display format",
                &stamp,
            ));
        }
        if let Some(Value::Text(language)) = read(link, 0x2E, &[]) {
            device_values.push(reading(
                "language",
                "Settings",
                "Language",
                language,
                "Watch interface language",
                &stamp,
            ));
        }
        if let Some(Value::Bytes(quiet)) = read(link, 0x2C, &[]) {
            if quiet.len() >= 4 {
                let start = u16::from_le_bytes([quiet[0], quiet[1]]);
                let end = u16::from_le_bytes([quiet[2], quiet[3]]);
                device_values.push(reading(
                    "quiet-hours",
                    "Settings",
                    "Quiet hours",
                    format!("{}–{}", hhmm(start), hhmm(end)),
                    "Do-not-disturb window",
                    &stamp,
                ));
            }
        }
        if let Some(Value::Bytes(alarms)) = read(link, 0xB9, &[0x15]) {
            let count = alarms.get(2).copied().unwrap_or(0);
            device_values.push(reading(
                "alarms",
                "Alarms",
                "Configured alarms",
                if count == 0 {
                    "None set".to_string()
                } else {
                    format!("{count} set")
                },
                "Alarms stored on the watch",
                &stamp,
            ));
        }
        if let Some(Value::Text(value)) = read(link, 0x83, &[]) {
            cards.push(reading(
                "move-reminders",
                "Today",
                "Move reminders",
                value,
                "Reminder schedule stored on the watch",
                &stamp,
            ));
        }
        if let Some(Value::Bytes(apps)) = read(link, 0xB9, &[0x14]) {
            // B9{14,00} (decimal 20) is live-verified from the 2026-11-23 probe:
            // the trailing bytes are app IDs, not NUL-separated names.
            let value = decode_message_apps(&apps)
                .map(|list| list.label())
                .unwrap_or_default();
            device_values.push(reading(
                "message-apps",
                "Messages",
                "Watch message apps",
                value,
                "Notification apps enabled on the watch",
                &stamp,
            ));
        }
        if let Some(Value::Bytes(layout)) = read(link, 0x39, &[]) {
            if layout.len() >= 16 {
                let width = u16::from_be_bytes([layout[0], layout[1]]);
                let height = u16::from_be_bytes([layout[2], layout[3]]);
                let compression = String::from_utf8_lossy(&layout[8..16]).trim().to_string();
                device_values.push(reading(
                    "face-layout",
                    "Faces",
                    "Face layout",
                    format!("{width} × {height}"),
                    "Installed watch-face geometry",
                    &stamp,
                ));
                device_values.push(reading(
                    "face-compression",
                    "Faces",
                    "Face format",
                    compression,
                    "Installed watch-face encoding",
                    &stamp,
                ));
            }
        }

        if let Some(store) = self.store.as_mut() {
            for value in cards.iter().chain(device_values.iter()) {
                store.put_reading(value.key, &value.value, at)?;
            }
            for (direction, hex, cmd) in &frames {
                store.put_frame(direction, hex, at, registry::confidence(*cmd))?;
            }
        }
        if let Some(value) = battery {
            self.battery = value;
        }
        self.cards = cards;
        self.device_values = device_values;
        self.last_sync = Some(stamp);
        Ok(())
    }

    /// Build a write frame from the action and send it through the connection
    /// manager's write path. No raw frame is ever rendered on Today or Device.
    pub fn act(&mut self, kind: Action) -> Result<(), AppError> {
        let link: &dyn DeviceLink = self
            .link
            .as_ref()
            .map(|link| link.as_ref())
            .ok_or(AppError::NoLink)?;
        let (frame, updated): (Vec<u8>, Option<(&'static str, String)>) = match kind {
            Action::SetGoal(steps) => (
                command::set_goal_steps(steps),
                Some(("goals-steps", steps.to_string())),
            ),
            Action::SetTimeSystem { h24 } => (
                command::set_time_system(h24),
                Some(("time-system", if h24 { "24h" } else { "12h" }.to_string())),
            ),
            Action::SetDnd { start, end, enable } => (
                command::set_dnd(start, end, enable),
                Some((
                    "quiet-hours",
                    if enable {
                        format!(
                            "{}–{}",
                            hhmm(u16::from(start.0) * 60 + u16::from(start.1)),
                            hhmm(u16::from(end.0) * 60 + u16::from(end.1))
                        )
                    } else {
                        "Off".to_string()
                    },
                )),
            ),
            Action::SetAlarm {
                slot,
                hour,
                minute,
                repeat,
                date,
            } => (
                command::set_alarm(slot, hour, minute, repeat, date),
                Some(("alarms", format!("{hour:02}:{minute:02}"))),
            ),
            Action::DeleteAlarm(slot) => (command::delete_alarm(slot), None),
            Action::Weather {
                cond,
                temp,
                city,
                festival,
            } => (command::weather_today(cond, temp, &city, &festival), None),
            Action::Message(text) => (command::message(&text), None),
            Action::TimeSync(unix) => (command::time_sync(unix), None),
            Action::StartHr => (command::start_hr(), None),
        };
        link.write(&frame)?;
        let at = (self.now)();
        let stamp = utc_label(at);
        self.last_sync = Some(stamp.clone());
        if let Some(store) = self.store.as_mut() {
            let cmd = frame.get(4).copied().unwrap_or(0);
            store.put_frame("tx", &hex_of(&frame), at, registry::confidence(cmd))?;
        }
        if let Some((key, value)) = updated {
            for reading in self.cards.iter_mut().chain(self.device_values.iter_mut()) {
                if reading.key == key {
                    reading.value = value.clone();
                    reading.updated_at = stamp.clone();
                }
            }
            if let Some(store) = self.store.as_mut() {
                store.put_reading(key, &value, at)?;
            }
        }
        Ok(())
    }

    /// First step: arm the control for the card at `index`. Opens the
    /// confirmation sheet but writes nothing until the second click.
    pub fn open_control_sheet(&mut self, index: usize) -> bool {
        let Some(action) = self.cards.get(index).and_then(action_for) else {
            return false;
        };
        self.armed_action = Some(action);
        self.control_sheet_open = true;
        true
    }

    /// Dismiss the sheet without writing.
    pub fn close_control_sheet(&mut self) {
        self.armed_action = None;
        self.control_sheet_open = false;
    }

    /// Second step: send the armed action through the device link and close
    /// the sheet. A confirm without a prior arm is a no-op.
    pub fn confirm_control_action(&mut self) -> Result<(), AppError> {
        let Some(action) = self.armed_action.take() else {
            self.control_sheet_open = false;
            return Ok(());
        };
        self.control_sheet_open = false;
        let result = self.act(action);
        // A failed write must reach the status strip, like a failed refresh.
        if let Err(ref error) = result {
            self.failed = true;
            self.last_error = Some(error.to_string());
        }
        result
    }

    pub fn control_sheet_open(&self) -> bool {
        self.control_sheet_open
    }

    /// True when the reading key has an L-verified write mapping.
    pub fn card_writable(key: &str) -> bool {
        matches!(key, "goals-steps" | "heart-rate")
    }

    pub(crate) fn apply_control_sheet(&self, window: &AppWindow) {
        window.set_control_sheet_open(self.control_sheet_open);
        let label = self
            .armed_action
            .as_ref()
            .map(action_label)
            .unwrap_or_default();
        window.set_control_sheet_label(SharedString::from(label.as_str()));
    }

    /// Publish the UI-owned refresh state. Called on sync start as well as in
    /// `apply_to`, so the strip changes before a blocking read begins.
    pub(crate) fn apply_sync(&self, window: &AppWindow) {
        window.set_syncing(self.syncing);
        window.set_sync_phase(SharedString::from(self.sync_phase().label()));
        window.set_sync_failed(self.failed);
        window.set_last_error(SharedString::from(self.last_error.as_deref().unwrap_or("")));
    }

    pub(crate) fn apply_to(&self, window: &AppWindow, options: &UiOptions) {
        window.set_device_name(SharedString::from(self.device_name.as_str()));
        window.set_battery(self.battery as i32);
        window.set_connection_state(SharedString::from(connection_label(self.connection)));
        window.set_last_sync(SharedString::from(
            last_sync_label(self.last_sync.as_deref()).as_str(),
        ));
        self.apply_sync(window);
        window.set_reduced_transparency(options.reduced_transparency);
        window.set_high_contrast(options.high_contrast);
        self.apply_cards(window);
        self.apply_device_values(window);
        self.apply_control_sheet(window);
    }

    pub(crate) fn apply_cards(&self, window: &AppWindow) {
        let cards: Vec<crate::CardView> = self
            .cards
            .iter()
            .map(|value| crate::CardView {
                key: SharedString::from(value.key),
                title: SharedString::from(value.title),
                value: SharedString::from(value.value.as_str()),
                detail: SharedString::from(value.detail),
                updated_at: SharedString::from(value.updated_at.as_str()),
                writable: Self::card_writable(value.key),
            })
            .collect();
        window.set_metric_cards(ModelRc::new(VecModel::from(cards)));
    }

    pub(crate) fn apply_device_values(&self, window: &AppWindow) {
        let rows: Vec<crate::RowView> = self
            .device_values
            .iter()
            .map(|value| crate::RowView {
                group: SharedString::from(value.group),
                title: SharedString::from(value.title),
                value: SharedString::from(value.value.as_str()),
                detail: SharedString::from(value.detail),
                updated_at: SharedString::from(value.updated_at.as_str()),
            })
            .collect();
        window.set_device_values(ModelRc::new(VecModel::from(rows)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_states_have_human_labels() {
        assert_eq!(connection_label(ConnectionState::Idle), "Idle");
        assert_eq!(connection_label(ConnectionState::Connected), "Connected");
        assert_eq!(connection_label(ConnectionState::NotFound), "Not found");
    }

    #[test]
    fn missing_last_sync_reads_never() {
        assert_eq!(last_sync_label(None), "Never synced");
        assert_eq!(
            last_sync_label(Some("2026-11-22 14:03 UTC")),
            "Synced 2026-11-22 14:03 UTC"
        );
    }

    use std::cell::RefCell;
    use std::rc::Rc;

    use openwatch_core::ble::BleError;
    use openwatch_core::protocol::frame::{packet, Frame};

    fn fixed_now() -> i64 {
        1_795_356_180 // 2026-11-22 14:03 UTC
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn face_layout() -> Vec<u8> {
        let mut bytes = vec![0u8; 46];
        bytes[0..2].copy_from_slice(&240u16.to_be_bytes());
        bytes[2..4].copy_from_slice(&284u16.to_be_bytes());
        bytes[4..6].copy_from_slice(&140u16.to_be_bytes());
        bytes[6..8].copy_from_slice(&163u16.to_be_bytes());
        bytes[8..16].copy_from_slice(b"ORIGINAL");
        bytes
    }

    /// The 20 live read fixtures captured from the Fire-Boltt 039 sweep.
    fn live_fixtures() -> Vec<(u8, Vec<u8>)> {
        let goals = vec![
            0x10, 0x00, 0x01, 0x10, 0x27, 0x00, 0x00, 0x02, 0x2C, 0x01, 0x00, 0x00, 0x03, 0x64,
            0x20, 0x00, 0x00, 0x04, 0x1E, 0x00, 0x00, 0x00,
        ];
        vec![
            (0x2B, vec![0, 0, 0, 0x1D, 0xF3]), // steps today: 7667
            (0x2E, vec![1]),                   // language: international
            (0xBA, vec![1, 0, 40]),            // firmware: 1.0.40
            (0x29, b"international".to_vec()), // edition
            (0xBB, vec![0x0B, 0xF0, 0x00]),    // message limit: 240
            (0x84, vec![1]),                   // device state
            (0x35, vec![0; 73]),               // HR history
            (0x59, vec![0; 49]),               // step categories
            (0x2F, vec![0]),                   // auto-HR index
            (0x81, vec![72]),                  // live HR a
            (0x82, vec![72]),                  // live HR b
            (0x27, vec![1]),                   // time system: 24h
            (0x2C, vec![0, 0, 0xA4, 0x01]),    // quiet hours 00:00-07:00
            (0x39, face_layout()),             // watch-face layout
            (0xB9, goals),                     // goals
            (0xB9, vec![0x11, 1, 2, 3, 4]),    // stress
            (0xB9, vec![0x15, 0x04, 0x00]),    // alarms: none set
            // B9{14,00} message-app list, live capture 2026-11-23 (19 entries).
            (
                0xB9,
                hex_bytes("140013000102030405060708090a0b0c0d0e0f111f80"),
            ),
            // 0x83 move reminders, live capture 2026-11-23.
            (0x83, hex_bytes("3c320a16")),
            (0x26, vec![0x10, 0x27, 0, 0]), // step goal readback
            (0x64, vec![0]),                // weather probe
        ]
    }

    #[derive(Debug)]
    struct MockLink {
        replies: Vec<(u8, Vec<u8>)>,
        writes: Rc<RefCell<Vec<Vec<u8>>>>,
        battery: u8,
    }

    impl DeviceLink for MockLink {
        fn query(&self, cmd: u8, payload: &[u8], _wait_ms: u64) -> Result<Frame, AppError> {
            let sub = payload.first().copied();
            self.replies
                .iter()
                .find(|(c, p)| {
                    *c == cmd && (cmd != 0xB9 || sub.is_none() || p.first() == sub.as_ref())
                })
                .map(|(c, p)| Frame {
                    cmd: *c,
                    payload: p.clone(),
                })
                .ok_or_else(|| BleError::Timeout.into())
        }

        fn write(&self, frame: &[u8]) -> Result<(), AppError> {
            self.writes.borrow_mut().push(frame.to_vec());
            Ok(())
        }

        fn battery(&self) -> Option<u8> {
            Some(self.battery)
        }

        fn connection(&self) -> ConnectionState {
            ConnectionState::Connected
        }
    }

    #[test]
    fn refresh_reads_populates_today_and_device_with_timestamps() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let link = MockLink {
            replies: live_fixtures(),
            writes: writes.clone(),
            battery: 18,
        };
        let mut state = AppState::with_link(link, fixed_now);
        state.refresh_reads().unwrap();

        assert_eq!(state.battery, 18);
        assert_eq!(state.reading("firmware").unwrap().value, "1.0.40");
        assert_eq!(state.reading("goals-steps").unwrap().value, "10000");
        assert_eq!(state.reading("message-limit").unwrap().value, "240");
        assert_eq!(state.reading("message-apps").unwrap().value, "19 enabled");
        assert_eq!(
            state.reading("move-reminders").unwrap().value,
            "50 steps every 60 min · 10:00–22:00"
        );
        assert!(!state.cards.is_empty());
        assert!(!state.device_values.is_empty());
        for card in state.cards.iter().chain(state.device_values.iter()) {
            assert_eq!(card.updated_at, "2026-11-22 14:03 UTC", "{}", card.key);
            assert!(
                !card.value.to_lowercase().contains("feea"),
                "raw hex leaked into a human-readable surface: {}",
                card.key
            );
        }
    }

    fn soon_now() -> i64 {
        fixed_now() + 299
    }

    fn late_now() -> i64 {
        fixed_now() + 301
    }

    #[test]
    fn sync_phase_goes_stale_five_minutes_after_the_last_refresh() {
        let link = MockLink {
            replies: live_fixtures(),
            writes: Rc::new(RefCell::new(Vec::new())),
            battery: 18,
        };
        let mut state = AppState::with_link(link, fixed_now);
        state.refresh_reads().unwrap();
        state.sync_connection();
        assert_eq!(state.connection, ConnectionState::Connected);
        assert_eq!(state.sync_phase(), SyncPhase::Connected);

        state.now = soon_now;
        assert_eq!(state.sync_phase(), SyncPhase::Connected);
        state.now = late_now;
        assert_eq!(state.sync_phase(), SyncPhase::Stale);
        assert_eq!(state.sync_phase().label(), "Stale · sync now");
    }

    #[test]
    fn refresh_logs_exact_frames_for_both_directions() {
        let store = Store::open(":memory:").unwrap();
        let link = MockLink {
            replies: live_fixtures(),
            writes: Rc::new(RefCell::new(Vec::new())),
            battery: 18,
        };
        let mut state = AppState::with_link(link, fixed_now).with_store(store);
        state.refresh_reads().unwrap();

        let frames = state.stored_frames(500).unwrap();
        assert!(!frames.is_empty(), "no frames were logged");
        assert!(frames.iter().any(|row| row.direction == "tx"));
        assert!(frames.iter().any(|row| row.direction == "rx"));
        for row in &frames {
            assert_eq!(row.at_unix, fixed_now());
            assert!(matches!(
                row.confidence.as_str(),
                "live" | "source" | "unverified"
            ));
            let (cmd, opcode) = crate::data_view::opcode_for(&row.frame_hex);
            assert_ne!(cmd, "—", "inspector cannot read {}", row.frame_hex);
            assert_ne!(
                opcode, "unparsed",
                "inspector cannot read {}",
                row.frame_hex
            );
        }
    }

    #[test]
    fn sync_connection_refreshes_the_status_strip() {
        let link = MockLink {
            replies: vec![],
            writes: Rc::new(RefCell::new(Vec::new())),
            battery: 18,
        };
        let mut state = AppState::with_link(link, fixed_now);
        assert_eq!(state.connection, ConnectionState::Idle);
        state.sync_connection();
        assert_eq!(state.connection, ConnectionState::Connected);
    }

    #[test]
    fn set_goal_action_writes_exact_frame_through_link() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let link = MockLink {
            replies: vec![],
            writes: writes.clone(),
            battery: 18,
        };
        let mut state = AppState::with_link(link, fixed_now);
        state.act(Action::SetGoal(8000)).unwrap();
        let recorded = writes.borrow();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], hex_bytes("feea20091600001f40"));
    }

    #[test]
    fn cards_can_be_reordered_without_losing_timestamps() {
        let link = MockLink {
            replies: live_fixtures(),
            writes: Rc::new(RefCell::new(Vec::new())),
            battery: 18,
        };
        let mut state = AppState::with_link(link, fixed_now);
        state.refresh_reads().unwrap();
        let before: Vec<&str> = state.cards.iter().map(|card| card.key).collect();
        assert!(before.len() >= 2);
        let first = before[0].to_string();
        state.move_card(&first, 1);
        let after: Vec<&str> = state.cards.iter().map(|card| card.key).collect();
        assert_eq!(after[1], first);
        assert!(state.cards.iter().all(|card| !card.updated_at.is_empty()));
    }

    #[test]
    fn utc_label_formats_epoch_seconds() {
        assert_eq!(utc_label(1_795_356_180), "2026-11-22 14:03 UTC");
    }

    #[test]
    fn query_uses_a_complete_control_frame() {
        // Guard the fixture helper: frames must round-trip through the codec.
        let bytes = packet(0x16, &8000u32.to_be_bytes());
        assert_eq!(bytes, hex_bytes("feea20091600001f40"));
    }
}
