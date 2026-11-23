# OpenWatch — design document

_Date: 2026-11-22. Status: proposed. Product/repo: `sahilwasgood/OpenWatch`._

## 1. Intent

Build a local-first, cross-platform smartwatch companion and protocol workbench for the
Fire-Boltt 039 (MOY-J3W2, FE EA / JieLi CRP) and, later, the Noise NSW-434 (ZH_SDK). The
operator wants everything the vendor app shows, everything the vendor app hides, and the
ability to change anything the watch exposes — without an account, cloud lock-in, paywall,
or a dumbed-down "wellness" surface.

The first screen is a **human-readable dashboard**: fitness data, device state, and controls,
all legible at a glance and all editable. **True raw protocol data lives in its own section**,
not on the main screen. An optional AI interpretation layer explains what values and frames
mean; cloud inference is acceptable and must be explicitly opt-in.

Success: one native binary per platform, same look and capability everywhere, no telemetry,
no login, verifiable protocol handling, and a public repo that another developer could extend.

## 2. Goals / non-goals / anti-goals

**Goals**
- Linux, macOS, Windows, Android from one Rust workspace.
- Fast, small, native; no Electron-sized webview bundle unless the UI choice demands it.
- Full feature parity with the current Python toolchain, plus the legacy console features
  that are recoverable from the ledger/source notes.
- Two views of the same data: readable dashboard (default) and raw protocol inspector (separate).
- Confidence labeling on every capability: `L` live-verified, `S` source-derived, `U` unverified.
- Local persistence; user can export and delete everything.
- Every state-changing action requires an explicit user action; nothing silent or scheduled.
- Continuous improvement loop: research → implement → test → review → fix → persist.

**Non-goals**
- Phone-side OS integration that needs system signatures (call answering, media control on iOS).
- Firmware/OTA flashing in v1 (protocol exists but is unverified and destructive).
- NF2 advanced commands (blocked on official-app HCI capture).
- Multi-user accounts, sync servers, subscriptions, ads.

**Anti-goals (what makes this a failure even if it compiles)**
- A UI that looks generated: gradient blobs, purple glows, emoji-as-icons, template cards.
- Locking readings behind taps or hiding controls "for safety".
- Presenting source-derived guesses as verified facts.
- Sending health or frame data to any cloud service without an explicit, visible opt-in.

## 3. Constraints and decisions made here

These are the builder's calls, recorded so they are not re-litigated:

| Choice | Decision | Why |
|---|---|---|
| UI framework | **Slint 1.19+** (Skia backend) | Only stack with official Android support *and* a native renderer; `backdrop-blur` gives real glass on all four targets; declarative `.slint` avoids boilerplate look. Runner-up: Tauri 2 if web tooling becomes decisive. |
| BLE | **btleplug 0.13.x + Tokio** | Only library covering Windows/macOS/Linux/Android from Rust with discovery, read, write, subscribe and notifications. |
| Bluetooth core | `crates/core` defines `BleTransport` (subscribe returns a `Stream`), `DeviceKey` resolver | macOS never exposes the MAC — scan-and-match by advertised name there; use address directly elsewhere. |
| License | **GPL-3.0-or-later** | Slint's OSS license is GPL-3.0; prior art in this space is GPL/AGPL. Compatible and honest. |
| Repo visibility | **Public** | "Open source" was explicit; public gives free Actions, security features, arm64 runners. |
| AI layer | Optional, opt-in, **bring-your-own-key** against a configurable OpenAI-compatible endpoint | No OpenWatch account or service; feature is off with no key. Only the user-selected value/frame text plus short context is sent, and the exact payload is shown before sending. Raw frame always shown beside interpretation. |
| Weather source | **Manual entry in v1**; pluggable provider interface only | Keeps the no-cloud promise; a live provider would add a key and a network dependency for one feature. |
| Firmware verdict | Show **installed version** and a neutral "not checked" state; no automatic "up to date" claim in v1 | "Latest" needs either a vendor endpoint (cloud) or a bundled manifest that goes stale; neither is worth a false promise. |
| Notifications | **Watch-side message-app list** (`B9{20,0}`) in v1; OS notification forwarding deferred | The watch-side list is a known read/write setting; OS forwarding is a separate cross-platform subsystem per platform. |
| Face designer | **Raw face upload + preview in the advanced section** in v1; WYSIWYG designer deferred | Face upload is unverified (`U`); a designer needs verified asset/format behavior first. |
| Data store | Structured local file store (SQLite via `rusqlite`), UTC timestamps | Avoids the comma-separated-string and fake-epoch anti-patterns seen in prior art. |
| Naming | OpenWatch; crate names `openwatch-core`, `openwatch-desktop`, `openwatch-android` | One product name everywhere. |

## 4. Architecture

```
OpenWatch/
├── Cargo.toml                 [workspace]
├── crates/
│   ├── core/                  protocol, codecs, decoders, model, store, BleTransport trait
│   ├── desktop/               Slint host for Linux/macOS/Windows
│   └── android/               Slint Android activity + permissions
├── docs/
│   ├── superpowers/specs/     this document
│   ├── protocol/              per-opcode notes, copied from the ledger, with confidence tags
│   └── research/              summarized findings from the 8 research lanes
├── .github/workflows/         ci.yml (matrix), release.yml (tag app-v*)
├── README.md CONTRIBUTING.md LICENSE CODE_OF_CONDUCT.md SECURITY.md CHANGELOG.md
```

Rules: `core` has no GUI or platform dependency; frontends are thin. Decoders are pure
functions with offline fixture tests. Every write goes through one command path that logs
the exact frame, timestamp, and confidence tag.

## 5. Feature inventory (port checklist)

### Core protocol — L
- Frame `FE EA [0x20+(n>>8)] [n] cmd payload…`, `n = 5 + len(payload)`, no CRC on control frames.
- GATT: service `FEEA`, write `FEE2`, notify `FEE3`, file `FEE6`, live-steps `FEE1`, battery `2A19` (+100 = charging).
- Adapter self-heal on Linux; ~350 ms spacing between writes; never auto-retry message/camera.
- Decoders: `2B 2E BA 29 BB 35 59 B9{11,16,21,17} 81 82 27 2C 2F 39`.

### Dashboard reads (20) — L
Language mask, edition, firmware, face index, message limit, face list, HR history, step
categories, stress state/timestamp/hourly, goals, alarms, quiet hours, time system,
other-message state, watch-face layout, auto-HR index, battery.

### Dashboard writes / actions
- Time sync `0x31` — L, fire-and-forget.
- Message push `0x41 {0x80, UTF-8 ≤240 B}` — L, transient, never retried.
- Step goal `0x16` BE32 (query `0x26` LE32) — L; watch recomputes a derived goal on write.
- Time system `0x17`, DND `0x71`, auto-HR `0x1F` — L (readback-confirmed). Quiet hours `0x2C` is L for the read only (listed above under Dashboard reads); no separate quiet-hours write has been captured — `0x71` (DND) is the verified write path.
- Alarm set/delete `B9{05,00|02,…}` — L (dump-confirmed).
- Weather `0x43` today / `0x42` 7-day / `0xB5` sun; `0x64` is watch→phone only — S.
  Values are entered manually in v1; no weather provider is bundled.
- Live HR `0x6D {00}` → `{bpm}` or `{FF}` busy — L.
- Face upload, DFU, AGPS, OTA — U; exposed in the advanced section behind explicit confirmation.

### Separate sections
- **Raw inspector**: exact frames hex + ASCII, timestamp, direction, opcode name, confidence.
- **AI interpretation**: opt-in explanation of a selected value or frame; shows source frame beside text.
- **Legacy console features to recover from ledger/source**: camera `0x66`, ring `0x62`,
  move reminders `0x83`, women's health `0x85`, message-app list `B9{20,0}`, stress chart,
  continuous-HR live list, diagnostics sweep, OTA check, AGPS, face designer. All tagged `U`
  until live-verified; never silently presented as working. A full WYSIWYG face designer is
  deferred — v1 offers raw upload plus a layout preview derived from `0x39`.

### Device support
- Fire-Boltt 039: full support.
- Noise NSW-434: scaffold only (the engine is known; advanced commands need an app HCI capture).

## 6. UI/UX specification

**Information architecture** — four destinations, no nested maze:
1. **Today** — reorderable metric cards (steps, HR, battery, sleep/stress), connection strip
   (battery, last sync, Sync now), and a "Control" affordance on every card so the dashboard is
   editable, not read-only.
2. **Device** — identity, installed firmware, all settings, watch-side message-app list, alarms,
   goals, faces, and firmware version (no automatic verdict in v1).
3. **Workbench** — raw inspector, AI interpretation panel, advanced/mod opcodes, diagnostics.
4. **Data** — history, charts, local store, export/delete, units.

**Visual language** (from the research spec):
- Zinc/neutral ramp, one cool blue accent; status green/amber/red. Color on surfaces, never labels.
- Glass (`backdrop-filter blur(20px) saturate(140%)`, 1 px light edge, 24 px card radius) for
  **chrome and cards only**; numbers and long text on near-opaque content surfaces.
- Refraction only on 1–2 hero surfaces; 250 ms `cubic-bezier(.2,.8,.2,1)` transitions, 120 ms press.
- Tabular numerals, 11/13/15/17/22/34 pt scale, 4 pt spacing base.
- Explicit `reduced-transparency` → solid surfaces, `reduced-motion` → cross-fade, high-contrast
  and forced-colors fallbacks. Text contrast ≥4.5:1 measured against the lightest pixel after blur.

**Trust cues**: every value shows its last-update time; every write shows the exact frame in the
workbench; source-derived features carry a visible "unverified" tag; sync never fails silently.

## 7. Backend and reliability

- One async event loop (Tokio); one connection manager per device with exponential-backoff reconnect.
- Bounded/battery-gated background sessions; no constant polling when the watch is idle.
- Notification stream drained by a dedicated task; commands serialized through one writer with
  350 ms spacing; no automatic retry of transient writes.
- Local SQLite store; UTC normalization; export to JSON/CSV; full local delete.
- Confidence registry: a table mapping opcode → `L|S|U` → decoder → writer → test fixture.
- All cloud calls (AI interpretation only) are opt-in, show the exact payload, and are never
  required for any local feature.

## 8. Delivery plan

1. Repo scaffolding, workspace, CI, docs, license.
2. `openwatch-core`: framing, decoders, fixtures, command registry, store.
3. `openwatch-desktop`: Slint UI shell, Today + Device surfaces, BLE connection manager.
4. Workbench: raw inspector, AI panel, advanced actions.
5. Android packaging and platform permissions.
6. Feature parity pass against the checklist, then continuous loop: research → implement →
   test → review → fix → persist.

## 9. Decisions and live verification

**Operator decision:** exercise every command on the real watch and promote it to live-verified.
State-changing writes are approved for the purposes of verification; where practical, restore the
original value afterward and record before/after reads. The build loop includes a hardware pass
that updates the confidence registry from `S`/`U` to `L` with captured frames and timestamps.

The hardware pass is the ignored harness at `crates/core/tests/live/verify.rs`, driven by
`cargo test -p openwatch-core --test verify -- --ignored --nocapture` with the additional
explicit `OPENWATCH_LIVE_VERIFY=1` opt-in. It runs the 20 dashboard reads plus battery, exercises
the verified writes (restoring originals where practical), and writes exact frames with UTC
timestamps to `docs/protocol/verification-log-generated.md`. A non-ignored guard test asserts
the harness stays ignored, so a plain `cargo test` never touches the watch. Destructive
file-channel commands (face upload, DFU, AGPS, OTA) are never sent by the harness.

Recorded calls, not questions: Fire-Boltt parity comes before NSW-434 protocol work (NSW stays
scaffolded); BYOK AI with no account; manual weather in v1; installed-firmware-only with no
automatic verdict; watch-side message-app list instead of OS forwarding; raw face upload instead
of a WYSIWYG designer.
