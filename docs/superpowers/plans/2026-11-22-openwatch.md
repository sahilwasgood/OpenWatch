# OpenWatch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first, cross-platform Rust smartwatch companion with a human-readable dashboard, a separate raw protocol workbench, and full read/write control of the Fire-Boltt 039, with every opcode verified on real hardware.

**Architecture:** A pure `openwatch-core` crate owns framing, decoders, command builders, confidence registry, local storage and a `BleTransport` trait. Thin Slint frontends for desktop and Android consume it through a connection manager that serializes writes and handles reconnect. The UI has four destinations — Today, Device, Workbench, Data — with true raw frames confined to Workbench.

**Tech Stack:** Rust stable, Slint 1.19+ (Skia), btleplug 0.13.x, Tokio, rusqlite, serde, thiserror, tracing.

**Spec:** `docs/superpowers/specs/2026-11-22-openwatch-design.md`

## Global Constraints

- Frame: `FE EA [0x20+(n>>8)] [n] cmd payload…`, `n = 5 + len(payload)`; **no CRC on control frames**.
- GATT: service `FEEA`, write `FEE2`, notify `FEE3`, file `FEE6`, live-steps `FEE1`, battery `2A19` (+100 = charging).
- Write spacing **350 ms**; never auto-retry message or camera writes.
- Fire-Boltt MAC `67:31:E4:DD:50:7A`; on macOS resolve by advertised name instead.
- Local-first: no account, no telemetry, no cloud required for any local feature; SQLite store with UTC timestamps.
- AI interpretation is opt-in BYOK, off with no key; exact payload shown before send; source frame shown beside text.
- Weather values are manual in v1; firmware shows installed version only, no automatic verdict.
- Confidence tags on every capability: `L` live-verified, `S` source-derived, `U` unverified.
- License GPL-3.0-or-later; public repo `sahilwasgood/OpenWatch`.
- Operator approved exercising every command on real hardware; restore original value where practical.

## Review Focus

- **Watch asleep / not advertising:** connection manager must report "not found" and retry with backoff, never hang a UI thread. Test in Task 7.
- **macOS has no MAC address:** resolver must scan-and-match by advertised name, not fail on a MAC lookup. Test in Task 6.
- **Non-ASCII message text:** the `0x41` builder must cap payload by UTF-8 bytes (≤240), not by character count, and must not panic. Test in Task 4.
- **Keepalive interleaving:** notification handling must select the `FE EA` frame whose cmd matches the pending query, not the first `F9` keepalive. Test in Task 3.
- **AI with no key configured:** panel must be disabled, make zero network calls, and send no health data. Test in Task 10.

---

### Task 1: Workspace scaffolding and repo hygiene

**Files:**
- Create: `Cargo.toml`, `crates/core/Cargo.toml`, `crates/core/src/lib.rs`, `crates/desktop/Cargo.toml`, `crates/desktop/src/main.rs`, `crates/android/Cargo.toml`, `crates/android/src/lib.rs`
- Create: `.github/workflows/ci.yml`, `README.md`, `CONTRIBUTING.md`, `LICENSE`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`

**Interfaces:**
- Produces: a Cargo workspace whose members are `crates/core`, `crates/desktop`, `crates/android`; each crate compiles with `cargo check --workspace`.

- [ ] **Step 1: Write the workspace manifest and three crate manifests**

`Cargo.toml` declares `[workspace] members = ["crates/*"]`, `resolver = "2"`, and a `[workspace.package]` with `edition = "2021"`, `license = "GPL-3.0-or-later"`, `version = "0.1.0"`. `crates/core/Cargo.toml` depends on `serde`, `thiserror`, `tokio` (features `sync`, `time`), `rusqlite` (feature `bundled`), `tracing`. `crates/desktop/Cargo.toml` depends on `slint = "1.19"`, `tokio` (features `full`), and `openwatch-core` by path. `crates/android/Cargo.toml` depends on `slint` and `openwatch-core` by path.

- [ ] **Step 2: Add CI and repo files**

`.github/workflows/ci.yml` runs on push and PR with a matrix of `ubuntu-latest`, `macos-latest`, `windows-latest`; steps `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`. `README.md` states local-first/no-account, supported platforms, build commands, and links both design and research docs. `LICENSE` is GPL-3.0 text.

- [ ] **Step 3: Verify the build**

Run: `cargo check --workspace`
Expected: all three crates compile; no errors.

- [ ] **Step 4: Commit**

`git add -A && git commit -m "feat: workspace scaffolding and CI"`

---

### Task 2: Frame codec

**Files:**
- Create: `crates/core/src/protocol/mod.rs`, `crates/core/src/protocol/frame.rs`
- Test: inline `#[cfg(test)] mod tests` in `frame.rs`

**Interfaces:**
- Produces:
  - `pub fn packet(cmd: u8, payload: &[u8]) -> Vec<u8>`
  - `pub struct Frame { pub cmd: u8, pub payload: Vec<u8> }`
  - `pub fn parse(data: &[u8]) -> Result<Frame, ProtocolError>`
  - `pub enum ProtocolError { TooShort, BadMagic, BadLength }`

- [ ] **Step 1: Write failing codec tests**

Assert: `packet(0x53, &[]) == [0xFE,0xEA,0x20,0x05,0x53]`; `packet(0x66, &[0x01]) == [0xFE,0xEA,0x20,0x06,0x66,0x01]`; a 300-byte payload yields header `[0xFE,0xEA,0x21,0x31]`; `parse(&packet(0xB9,&[0x15,0x04]))` returns `Frame { cmd: 0xB9, payload: vec![0x15,0x04] }`; `parse(&[0x00,0x01])` is `Err(BadMagic)`; `parse(&[0xFE,0xEA,0x20,0x09,0x53])` is `Err(BadLength)`.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core frame`
Expected: FAIL — functions not found.

- [ ] **Step 3: Implement the codec**

`packet` computes `total = 5 + payload.len()`, `b2 = (0x20 + (total >> 8)) & 0xFF`, returns magic + b2 + low byte + cmd + payload. `parse` checks magic, minimum length, then that `data[3] as usize == data.len()`.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-core frame`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

`git add crates/core/src/protocol && git commit -m "feat: FE EA frame codec"`

---

### Task 3: Decoders and confidence registry

**Files:**
- Create: `crates/core/src/protocol/decode.rs`, `crates/core/src/protocol/registry.rs`
- Test: inline tests in both files

**Interfaces:**
- Consumes: `Frame` and `ProtocolError` from Task 2.
- Produces:
  - `pub enum Value { Number(u64), Text(String), Bytes(Vec<u8>), Flag(bool), Time(String) }`
  - `pub fn decode(cmd: u8, payload: &[u8]) -> Option<Value>`
  - `pub fn select_reply(frames: &[Frame], cmd: u8) -> Option<Frame>` — returns the first frame with matching `cmd`, ignoring keepalives.
  - `pub enum Confidence { Live, Source, Unverified }`
  - `pub fn confidence(cmd: u8) -> Confidence`
  - `pub fn all_opcodes() -> Vec<(u8, &'static str, Confidence)>`

- [ ] **Step 1: Write failing decoder tests**

Assert `decode(0x2E, &[1]) == Value::Text("international")`; `decode(0xBA, &[1,0,40]) == Value::Text("1.0.40")`; `decode(0x2B, &[0,0,0,0x1D,0xF3])` contains `0x00001DF3`; `decode(0xBB, &[0x0B,0xF0,0x00])` contains `240`; `decode(0x27, &[1]) == Value::Text("24h")`; `decode(0x39, &layout_46_bytes)` yields width 240, height 284, thumb 140×163, compression `ORIGINAL`. Assert `select_reply` skips a `0xF9` frame and returns the `0x2E` frame from a mixed slice.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core decode`
Expected: FAIL.

- [ ] **Step 3: Implement decoders**

Implement each arm using the exact layouts from the design doc. `0xB9` subdispatch: `0x10` goals as [type u8][LE u32] pairs starting at payload byte 2; `0x15` alarms as count at byte 2 and 8-byte records from byte 3; `0x11` stress state/timestamp/hourly. `select_reply` filters `frame.cmd == cmd`.

- [ ] **Step 4: Implement the registry with the exact confidence table**

`Live` for: `0x2B,0x2E,0xBA,0x29,0xBB,0x84,0x35,0x59,0x2F,0x81,0x82,0x27,0x2C,0x39`, `0xB9` subs `0x10,0x11,0x15`, writes `0x16,0x17,0x31,0x41,0x6D`. `Source` for: `0x43,0x42,0xB5,0x71,0x1F`, `0xB9` set/delete alarm. `Unverified` for: `0x66,0x62,0x83,0x85,0xB6`, face upload, DFU, AGPS, OTA.

- [ ] **Step 5: Run tests, verify pass**

Run: `cargo test -p openwatch-core decode registry`
Expected: PASS.

- [ ] **Step 6: Commit**

`git add crates/core/src/protocol && git commit -m "feat: decoders and confidence registry"`

---

### Task 4: Command builders

**Files:**
- Create: `crates/core/src/protocol/command.rs`
- Test: inline tests

**Interfaces:**
- Consumes: `packet` from Task 2.
- Produces:
  - `pub fn message(text: &str) -> Vec<u8>` — full frame for `0x41`, payload `0x80` + UTF-8 bytes capped at 240.
  - `pub fn time_sync(unix: u32) -> Vec<u8>` — `0x31` with LE u32 + `0x08`.
  - `pub fn set_goal_steps(steps: u32) -> Vec<u8>` — `0x16` BE u32.
  - `pub fn set_time_system(h24: bool) -> Vec<u8>` — `0x17`.
  - `pub fn set_dnd(start: (u8,u8), end: (u8,u8), enable: bool) -> Vec<u8>` — `0x71`.
  - `pub fn set_alarm(slot: u8, hour: u8, minute: u8, repeat: u8, date: (u16,u8,u8)) -> Vec<u8>` — `0xB9` with `[0x05,0x00,record]`.
  - `pub fn delete_alarm(slot: u8) -> Vec<u8>` — `0xB9` with `[0x05,0x02,slot]`.
  - `pub fn start_hr() -> Vec<u8>` — `0x6D` with `[0x00]`.
  - `pub fn weather_today(cond: u8, temp: i8, city: &str, festival: &str) -> Vec<u8>` — `0x43`.
  - `pub fn weather_forecast(cond: u8, max: i8, min: i8, days: &[(u8,i8,i8)]) -> Vec<u8>` — `0x42`, 24 bytes.
  - `pub fn weather_sun(cond: u8, temp: i8, sunrise: (u8,u8), sunset: (u8,u8), location: &str) -> Vec<u8>` — `0xB5`.

- [ ] **Step 1: Write failing builder tests**

Assert exact frames: `set_goal_steps(8000) == feea20091600001f40`; `set_time_system(true) == feea20061701`; `set_alarm(0,7,30,0,(2026,10,19)) == feea200fb90500000100071eba1300`; `delete_alarm(3) == feea2008b9050203`; `start_hr() == feea20066d00`; `message("HELLO") == feea200b418048454c4c4f`; weather today for sunny/28/"DELH" == `feea20184300051c002000200020002000440045004c0048`. Assert `message("é".repeat(300))` payload is exactly 241 bytes total (1 type byte + 240 UTF-8 bytes) and does not panic.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core command`
Expected: FAIL.

- [ ] **Step 3: Implement builders**

Use `packet` from Task 2. Weather names are exactly 4 UTF-16BE code units, space-padded. Alarm record is 8 bytes: slot, enabled, kind, hour, minute, `(((year-2015)&0xFF)<<4)|(month&0x0F)`, day, weekmask.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-core command`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add crates/core/src/protocol/command.rs && git commit -m "feat: command builders"`

---

### Task 5: Local store

**Files:**
- Create: `crates/core/src/store.rs`
- Test: inline tests using an in-memory SQLite database

**Interfaces:**
- Produces:
  - `pub struct Store { conn: rusqlite::Connection }`
  - `pub fn open(path: &str) -> Result<Store, StoreError>`
  - `pub fn put_reading(&mut self, key: &str, value: &str, at_unix: i64) -> Result<(), StoreError>`
  - `pub fn readings(&self, key: &str) -> Result<Vec<(i64,String)>, StoreError>`
  - `pub fn put_frame(&mut self, direction: &str, frame_hex: &str, at_unix: i64, confidence: Confidence) -> Result<(), StoreError>`
  - `pub fn frames(&self, limit: i64) -> Result<Vec<FrameRow>, StoreError>`
  - `pub fn export_json(&self) -> Result<String, StoreError>`
  - `pub fn delete_all(&mut self) -> Result<(), StoreError>`

- [ ] **Step 1: Write failing store tests**

Open `:memory:`; insert two readings with different UTC timestamps; assert `readings` returns them in ascending time order; insert a frame with `Confidence::Live`; assert `frames(10)` returns it; assert `export_json` contains both keys; assert `delete_all` leaves zero rows.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core store`
Expected: FAIL.

- [ ] **Step 3: Implement the store**

Create tables `readings(key TEXT, value TEXT, at_unix INTEGER)` and `frames(id INTEGER PRIMARY KEY, direction TEXT, frame_hex TEXT, at_unix INTEGER, confidence TEXT)`. Store UTC epoch seconds. `export_json` serializes both tables with serde.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-core store`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add crates/core/src/store.rs && git commit -m "feat: local SQLite store"`

---

### Task 6: BLE transport and device key resolution

**Files:**
- Create: `crates/core/src/ble/mod.rs`, `crates/core/src/ble/device_key.rs`, `crates/core/src/ble/btleplug_transport.rs`
- Test: inline tests in `device_key.rs`; a mock transport test in `mod.rs`

**Interfaces:**
- Consumes: nothing beyond Tasks 1–2.
- Produces:
  - `#[async_trait] pub trait BleTransport { async fn connect(&mut self, key: &DeviceKey) -> Result<(), BleError>; async fn write(&self, frame: &[u8]) -> Result<(), BleError>; async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>, BleError>; async fn read_battery(&self) -> Result<u8, BleError>; async fn disconnect(&self) -> Result<(), BleError>; }`
  - `pub enum DeviceKey { Address(String), AdvertisedName(String) }`
  - `pub fn resolve_key(platform: &str, configured: &str) -> DeviceKey` — on `"macos"` returns `AdvertisedName`, otherwise `Address`.
  - `pub struct MockTransport` implementing `BleTransport` for tests.

- [ ] **Step 1: Write failing resolver and mock tests**

Assert `resolve_key("linux","67:31:E4:DD:50:7A") == DeviceKey::Address(...)`; `resolve_key("macos","67:31:E4:DD:50:7A") == DeviceKey::AdvertisedName("FireBoltt")`; a mock `write` records the exact frame; a mock `subscribe` yields queued frames including `0xF9`.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core ble`
Expected: FAIL.

- [ ] **Step 3: Implement resolver and mock**

`resolve_key` matches on platform string; macOS uses advertised name `"FireBoltt"`. `MockTransport` stores writes in a `Mutex<Vec<Vec<u8>>>` and yields a `futures::stream::iter` of queued frames.

- [ ] **Step 4: Implement the btleplug transport**

Connect by `DeviceKey`: scan and match name on macOS, use address elsewhere. Discover service `FEEA`, characteristics `FEE2` write, `FEE3` notify, battery `2A19` on `180F`. `subscribe` maps `Peripheral::notifications()` into a `Stream<Item=Vec<u8>>`.

- [ ] **Step 5: Run tests, verify pass**

Run: `cargo test -p openwatch-core ble`
Expected: PASS.

- [ ] **Step 6: Commit**

`git add crates/core/src/ble && git commit -m "feat: BLE transport and device key resolution"`

---

### Task 7: Connection manager

**Files:**
- Create: `crates/core/src/connection.rs`
- Test: inline tests with `MockTransport`

**Interfaces:**
- Consumes: `BleTransport`, `DeviceKey` from Task 6; `select_reply` from Task 3.
- Produces:
  - `pub enum ConnectionState { Idle, Scanning, Connecting, Connected, Reconnecting, NotFound }`
  - `pub struct ConnectionManager<T: BleTransport>`
  - `pub fn new(transport: T, key: DeviceKey) -> Self`
  - `pub async fn connect(&mut self) -> Result<(), BleError>` — 3 attempts with exponential backoff 1 s, 2 s, 4 s; on total failure sets `NotFound`.
  - `pub async fn query(&self, cmd: u8, payload: &[u8], wait_ms: u64) -> Result<Frame, BleError>` — writes `packet(cmd,payload)`, waits for notifications, returns `select_reply`.
  - `pub async fn write(&self, frame: &[u8]) -> Result<(), BleError>` — serializes through a writer task with 350 ms spacing.
  - `pub fn state(&self) -> ConnectionState`

- [ ] **Step 1: Write failing tests**

With a mock that always fails to connect, assert `connect()` returns `Err` and `state() == NotFound`. With a mock that queues a `0xF9` frame then a matching `0x2E` reply, assert `query(0x2E, &[], 50)` returns the `0x2E` frame. Assert two consecutive `write` calls are spaced ≥350 ms (measure with `Instant` in the mock).

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core connection`
Expected: FAIL.

- [ ] **Step 3: Implement connection manager**

Use a `tokio::sync::Mutex` for the writer and an `mpsc` channel for notifications; drain in a spawned task; `query` waits up to `wait_ms` and calls `select_reply`. Backoff sleeps 1 s, 2 s, then 4 s.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-core connection`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add crates/core/src/connection.rs && git commit -m "feat: connection manager with backoff and write spacing"`

---

### Task 8: Desktop UI shell and theme

**Files:**
- Create: `crates/desktop/ui/app-window.slint`, `crates/desktop/ui/theme.slint`, `crates/desktop/src/main.rs`, `crates/desktop/src/bridge.rs`
- Test: inline compile-time test in `crates/desktop/src/main.rs` that verifies the generated Slint window type and its four destination names

**Interfaces:**
- Consumes: `ConnectionManager`, `Store`, `decode`, `confidence` from earlier tasks.
- Produces: `pub struct AppState` in `bridge.rs` exposed to Slint as properties; `pub fn run() -> Result<(), AppError>` in `main.rs`.

- [ ] **Step 1: Write failing UI build test**

Assert that `slint::include_modules!()` compiles `app-window.slint`, and that the window exposes four navigation destinations named `Today`, `Device`, `Workbench`, `Data`.

- [ ] **Step 2: Run test, verify failure**

Run: `cargo test -p openwatch-desktop ui`
Expected: FAIL.

- [ ] **Step 3: Implement theme and shell**

`theme.slint` defines zinc palette, one blue accent `#0A84FF` in dark mode, 4 pt spacing base, radii 14/24/28, glass card style using Slint `backdrop-blur` on translucent rectangles, and solid content surfaces. `app-window.slint` defines a sidebar/nav with four destinations and a content area. `bridge.rs` holds `AppState` with device name, battery, connection state, and last-sync time.

- [ ] **Step 4: Assert accessibility fallbacks exist**

In the same compile-time test, assert `theme.slint` defines solid-surface tokens for reduced transparency, a short cross-fade duration for reduced motion, and a high-contrast border token.

- [ ] **Step 5: Run test, verify pass**

Run: `cargo test -p openwatch-desktop ui`
Expected: PASS.

- [ ] **Step 6: Commit**

`git add crates/desktop && git commit -m "feat: Slint desktop shell and theme"`

---

### Task 9: Today and Device surfaces

**Files:**
- Modify: `crates/desktop/ui/app-window.slint`; create `crates/desktop/ui/today.slint`, `crates/desktop/ui/device.slint`
- Modify: `crates/desktop/src/bridge.rs`

**Interfaces:**
- Consumes: `Value` from Task 3, all read commands from Task 4, `Store` from Task 5.
- Produces: `AppState::refresh_reads()` and `AppState::act(kind: Action) -> Result<(), AppError>` where `Action` covers goal, time system, DND, alarm, weather, message, time sync, HR start.

- [ ] **Step 1: Write failing bridge tests**

With a mock connection returning the 20 live fixtures, assert `refresh_reads()` populates battery 18, firmware "1.0.40", goals steps 10000 and message limit 240, and that every card carries a last-update timestamp. Assert `act(Action::SetGoal(8000))` writes `feea20091600001f40`.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-desktop bridge`
Expected: FAIL.

- [ ] **Step 3: Implement Today and Device**

Today shows reorderable cards with a visible control affordance on each; Device shows identity, installed firmware, settings rows, watch-side message-app list, alarms, goals, faces. All values are human-readable; no hex on these two surfaces.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-desktop bridge`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add crates/desktop && git commit -m "feat: Today and Device surfaces"`

---

### Task 10: Workbench, raw inspector, and AI panel

**Files:**
- Create: `crates/desktop/ui/workbench.slint`, `crates/desktop/src/ai.rs`, `crates/desktop/src/data_view.rs`
- Test: inline tests in `ai.rs`

**Interfaces:**
- Consumes: `Store::frames` from Task 5, `confidence` from Task 3.
- Produces: `pub struct AiConfig { pub endpoint: String, pub key: Option<String> }`; `pub fn enabled(config: &AiConfig) -> bool`; `pub fn payload(frame_hex: &str, note: &str) -> String`; `pub async fn explain(config: &AiConfig, frame_hex: &str) -> Result<String, AiError>`.

- [ ] **Step 1: Write failing AI tests**

Assert `enabled` is false with `key: None` and true with a key; assert `payload` contains the exact frame hex and the note; with no key, assert `explain` returns `Err(AiError::Disabled)` and performs zero requests (use a mock HTTP function injected into the module).

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-desktop ai`
Expected: FAIL.

- [ ] **Step 3: Implement AI panel and raw inspector**

`explain` calls an OpenAI-compatible chat endpoint only when enabled; send only the selected frame and note. Workbench shows a frame table (time, direction, cmd, confidence, hex), a detail pane, and the AI panel with source frame beside explanation. Advanced actions are listed with `S`/`U` tags and require explicit confirmation.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-desktop ai`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add crates/desktop && git commit -m "feat: workbench, raw inspector, and opt-in AI panel"`

---

### Task 11: Data surface and legacy feature recovery

**Files:**
- Create: `crates/desktop/ui/data.slint`; modify `crates/core/src/protocol/command.rs` and `registry.rs`
- Test: extend command and registry tests

**Interfaces:**
- Consumes: `Store::export_json`, `delete_all`; `Confidence::Unverified` from Task 3.
- Produces: builders and registry entries for `0x66` camera, `0x62` ring, `0x83` move reminders, `0x85` women's health, `0xB9{20,0}` message-app list, `0xB6` day-pack probes — all tagged `Unverified`.

- [ ] **Step 1: Write failing tests**

Assert `all_opcodes()` contains entries for `0x66, 0x62, 0x83, 0x85, 0xB6` each with `Confidence::Unverified`; assert a camera builder emits the documented `0x66` frame.

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test -p openwatch-core registry`
Expected: FAIL.

- [ ] **Step 3: Implement the entries and Data surface**

Data surface: history charts, export JSON/CSV, delete-all with confirmation, units setting. Advanced/unverified actions never appear on Today or Device.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p openwatch-core registry`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add crates && git commit -m "feat: data surface and unverified legacy commands"`

---

### Task 12: Android packaging

**Files:**
- Modify: `crates/android/Cargo.toml`, `crates/android/src/lib.rs`; create `crates/android/manifest/AndroidManifest.xml`, `.github/workflows/android.yml`

**Interfaces:**
- Consumes: `openwatch-core` and the shared Slint UI components from Tasks 8–11.
- Produces: an Android library crate buildable with `cargo ndk` and an AAB workflow.

- [ ] **Step 1: Add manifest permissions**

`AndroidManifest.xml` declares `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT` for API 31+, `ACCESS_FINE_LOCATION` for older APIs, `INTERNET` only for the opt-in AI feature, `minSdkVersion 26`.

- [ ] **Step 2: Verify cross-compilation config**

Run: `cargo check -p openwatch-android --target aarch64-linux-android`
Expected: compiles, or reports only a documented missing NDK toolchain (record the exact message).

- [ ] **Step 3: Add the AAB workflow**

`.github/workflows/android.yml` uses `dtolnay/rust-toolchain@stable`, installs `cargo-ndk`, builds all four ABIs, assembles an AAB, uploads as an artifact on tag `app-v*`.

- [ ] **Step 4: Commit**

`git add crates/android .github/workflows/android.yml && git commit -m "feat: Android packaging"`

---

### Task 13: Live hardware verification pass

**Files:**
- Modify: `crates/core/src/protocol/registry.rs`, `docs/protocol/*.md`, `docs/superpowers/specs/2026-11-22-openwatch-design.md`
- Test: new `tests/live/verify.rs` (ignored by default, run with `--ignored` when hardware is present)

**Interfaces:**
- Consumes: every builder and decoder from Tasks 2–4; `ConnectionManager` from Task 7.
- Produces: confidence entries promoted to `Live` with captured frame hex and UTC timestamp; a generated `docs/protocol/verification-log.md`.

- [ ] **Step 1: Write the live verification harness**

An ignored test that connects to the watch, runs each read and write in sequence, records before/after values for state-changing commands, restores the original where practical, and writes captured frames to `docs/protocol/verification-log.md`.

- [ ] **Step 2: Run the read sweep against the live watch**

Run: `cargo test -p openwatch-core --test verify -- --ignored --nocapture`
Expected: 20 reads decode with the known values; battery read succeeds.

- [ ] **Step 3: Run write verification**

With the watch present, exercise goal, time system, DND, alarm set/delete, weather, message, time sync, auto-HR, camera, ring, and live HR; capture reply frames; restore original settings where practical. Promote each verified opcode to `Live` in the registry with its captured frame.

- [ ] **Step 4: Re-run the whole suite and commit evidence**

Run: `cargo test --workspace`
Expected: all tests pass; verification log lists each opcode with `L` and its frame.

- [ ] **Step 5: Commit**

`git add crates/core/src/protocol docs/protocol && git commit -m "feat: live-verified protocol registry"`

---

### Task 14: Continuous improvement loop

**Files:**
- Create: `docs/improvement-backlog.md`, `docs/research/YYYY-MM-DD-*.md` as discovered
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: all prior tasks.
- Produces: a repeatable cycle — research → implement → test → review → fix → persist — with each cycle recorded in the backlog and changelog.

- [ ] **Step 1: Create the backlog format**

Columns: finding, source, impact, status, owning task, verification. Seed it from the research summary's "avoid" list and the unverified opcode list.

- [ ] **Step 2: Run one improvement cycle**

Pick the highest-impact open backlog item, implement and test it, update the backlog and changelog, commit.

- [ ] **Step 3: Commit**

`git add docs && git commit -m "docs: improvement backlog and first cycle"`
