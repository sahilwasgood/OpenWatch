# OpenWatch UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fixed, partially inert OpenWatch UI with a responsive, accessible, research-led interface where every advertised control works and every surface has a designed empty/loading/error state.

**Architecture:** Keep the Rust/Slint split: `openwatch-core` owns protocol/state, `openwatch-ui` owns all `.slint` files and Rust bridge code, hosts stay thin. Changes are layered: control wiring first (functional), then tokens/typography/contrast, then responsive layouts, then states/polish, then icons/motion, then verification.

**Tech Stack:** Rust stable, Slint 1.19 (Skia), Tokio, existing core/UI/host crates.

**Spec:** `docs/superpowers/specs/2026-11-23-ui-redesign-design.md`

## Global Constraints

- All UI stays in `crates/ui/`; generated Slint types never cross crate boundaries.
- Accent `#0a84ff` with `#09090b` text on every accent fill; body text ≥4.5:1, UI boundaries ≥3:1.
- Zinc ramp only: `#09090b #18181b #27272a #3f3f46 #52525b #71717a #a1a1aa #e4e4e7 #fafafa`.
- Spacing 4/8/12/16/24/32; radii 6/8/12; dividers 1px; type 24/18/15/13/12/11 px; min interactive label 12px.
- Motion: hover 120ms, press 90ms, state 180ms, panel 220ms; one easing family; reduced-motion platform-driven.
- Min desktop window 900×600; min Android viewport 360dp; sidebar ≥700px, bottom nav <700px.
- No emoji icons, purple gradient, glow, nested glass, hero sections, or bounce easing.
- Glass only on connection strip and device card; solid surfaces elsewhere; solid fallbacks mandatory.
- local-first/no telemetry; GPL-3.0-or-later; all workspace tests + fmt + clippy must pass.

## Review Focus

- A control that looks clickable but writes nothing — pin with a mocked write test from a UI callback.
- Narrow viewport where the sidebar/button targets overflow or vanish — pin with layout assertions at 360dp.
- Accent-filled surfaces with light text — pin with contrast assertions covering all accent fills.
- Empty Workbench/Data rendering placeholder garbage — pin with empty-state string assertions.
- Sync failure leaving a stale "connected" strip — pin with a failed-refresh status assertion.

---

### Task 1: Wire write controls

**Files:**
- Modify: `crates/ui/src/lib.rs`, `crates/ui/src/bridge.rs`, `crates/ui/ui/today.slint`, `crates/ui/ui/app-window.slint`
- Test: inline tests in `crates/ui/src/lib.rs`

**Interfaces:**
- Consumes: `AppState::act(Action) -> Result<(), AppError>` (bridge.rs), `Action` variants, `DeviceLink::write`.
- Produces: `open_control_sheet(index)`, `close_control_sheet()`, `confirm_control_action()` callbacks; `control-sheet-open` property.

- [ ] **Step 1: Write failing test** — a callback on a Today control invokes `act(Action::SetGoal(8000))` through a `MockLink` and the exact frame `feea20091600001f40` is written; assert a second confirm click is required before any write.
- [ ] **Step 2: Run focused UI test, confirm failure.**
- [ ] **Step 3: Implement** a two-step control sheet in `AppState` plus Slint callbacks; only cards with L-verified write mappings show the control affordance; remove the inert badge from cards without a mapping.
- [ ] **Step 4: Run focused test, confirm pass.**
- [ ] **Step 5: Run `cargo fmt --all -- --check` and `cargo test --workspace`.**
- [ ] **Step 6: Commit** `feat: wire device write controls`.

### Task 2: Visual tokens, contrast and accessibility toggles

**Files:**
- Modify: `crates/ui/ui/theme.slint`, `crates/ui/ui/app-window.slint`, `crates/ui/ui/today.slint`, `crates/ui/ui/device.slint`, `crates/ui/ui/workbench.slint`, `crates/ui/ui/data.slint`, `crates/ui/src/lib.rs`

**Interfaces:**
- Consumes: theme token names from `theme.slint`.
- Produces: shared type scale tokens, `FontBody/FontLabel/FontMicro`, focus-ring token, reachable `reduced-transparency`/`high-contrast` toggles.

- [ ] **Step 1: Write failing tests** — assert (a) every accent fill uses `#09090b` text by source scan, (b) minimum interactive font-size token is 12px, (c) `reduced-transparency`/`high-contrast` callbacks exist and toggle `UiOptions` state.
- [ ] **Step 2: Run focused tests, confirm failure.**
- [ ] **Step 3: Implement** the token scale, replace ad-hoc sizes, apply accent/near-black text everywhere, add 40px minimum hit areas to reorder/control buttons, add accessible labels, add toggles in Data settings, add a visible focus ring.
- [ ] **Step 4: Run focused tests, confirm pass.**
- [ ] **Step 5: Run fmt + workspace tests.**
- [ ] **Step 6: Commit** `feat: unify visual tokens and accessibility`.

### Task 3: Responsive layout

**Files:**
- Modify: `crates/ui/ui/app-window.slint`, `crates/ui/ui/today.slint`, `crates/ui/ui/device.slint`, `crates/ui/ui/workbench.slint`, `crates/ui/ui/data.slint`, `crates/ui/src/lib.rs`

**Interfaces:**
- Produces: `narrow` property (true below 700px), bottom-nav component, pane content using layouts/proportional widths.

- [ ] **Step 1: Write failing tests** — (a) `AppWindow` declares no fixed 1040px/720px geometry, (b) a bottom-nav component exists and is shown when `narrow`, (c) pane widths are expressions of `parent.width` rather than absolute pixels.
- [ ] **Step 2: Run focused tests, confirm failure.**
- [ ] **Step 3: Implement** a shared pane header, sidebar/bottom-nav switch, and layout-based pane bodies that keep all controls ≥40px at 360dp.
- [ ] **Step 4: Run focused tests, confirm pass.**
- [ ] **Step 5: Run fmt + workspace tests.**
- [ ] **Step 6: Commit** `feat: responsive app shell`.

### Task 4: States, copy and charts

**Files:**
- Modify: `crates/ui/src/lib.rs`, `crates/ui/src/bridge.rs`, `crates/ui/src/ai.rs`, `crates/ui/src/data_view.rs`, `crates/ui/ui/app-window.slint`, `crates/ui/ui/workbench.slint`, `crates/ui/ui/data.slint`

**Interfaces:**
- Consumes: `syncing` flag, `refresh_reads()` result, `Store` frames/readings.
- Produces: `sync-phase` label, `sync-failed` flag, `last-error` string, designed empty/loading/error/success states.

- [ ] **Step 1: Write failing tests** — (a) failed refresh sets a visible failure state with "Sync failed — retry", (b) Workbench with no selection shows "Select a frame to inspect" and no `Detail ·  ·  ·`, (c) Data export has a Copy action and confirmation, (d) chart uses a min/max baseline and newest-N downsampling.
- [ ] **Step 2: Run focused tests, confirm failure.**
- [ ] **Step 3: Implement** the status mapping (core ConnectionState authoritative; UI syncing/failed flags; 5-minute stale threshold), designed states, copy action, placeholder corrections ("Never synced", title-case states), chart baseline/axis, and render the confirmed-action log or remove it.
- [ ] **Step 4: Run focused tests, confirm pass.**
- [ ] **Step 5: Run fmt + workspace tests.**
- [ ] **Step 6: Commit** `feat: designed states and clearer data views`.

### Task 5: Icons and motion polish

**Files:**
- Create: `crates/ui/icons/*.svg`
- Modify: `crates/ui/ui/theme.slint`, `crates/ui/ui/app-window.slint`, `crates/ui/ui/workbench.slint`, `crates/ui/ui/data.slint`

**Interfaces:**
- Consumes: theme tokens; `svg` Slint feature if needed.
- Produces: one coherent Svg icon set bound via `colorize`.

- [ ] **Step 1: Write failing test** — assert no emoji codepoints in `.slint` sources and that nav/action icons resolve via `@image-url` with `colorize`.
- [ ] **Step 2: Run focused test, confirm failure.**
- [ ] **Step 3: Implement** a minimal SVG set (home, device, workbench, data, sync, download, trash, confirm), apply motion tokens to hover/press/panel transitions, keep reduced-motion behavior.
- [ ] **Step 4: Run focused test, confirm pass.**
- [ ] **Step 5: Run fmt + workspace tests.**
- [ ] **Step 6: Commit** `feat: coherent icons and motion`.

### Task 6: Visual verification and documentation

**Files:**
- Modify: `docs/improvement-backlog.md`, `docs/HANDOFF.md`, `README.md`
- Test: headless screenshot/smoke check if Slint testing backend allows; otherwise source-level invariants.

- [ ] **Step 1: Attempt a headless render** of the four panes; if unavailable, record the exact limitation and keep the source-level checks.
- [ ] **Step 2: Assert** no known anti-slop tokens (purple gradient, emoji icons, glow) and that open accessibility/window items are updated in the backlog.
- [ ] **Step 3: Run** `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`.
- [ ] **Step 4: Commit** `docs: UI redesign verification and handoff`.
