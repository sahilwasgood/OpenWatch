# OpenWatch — handoff (2026-11-23)

## Where things are

- Repo `sahilwasgood/OpenWatch`.
- Main checkout: `/media/mint/WDC_WD10EZRX_932G/watch/OpenWatch` (branch `main`; doc-only commits incl. seeded backlog/handoff, endpoint probe, live-verify log).
- Implementation worktree: `/media/mint/WDC_WD10EZRX_932G/watch/OpenWatch/.worktrees/openwatch-impl`, branch `openwatch-impl`. **All implementation commits go here.**
- At Task 14 start: `openwatch-impl` HEAD `e38e097` (`feat: live-verified protocol registry`), tasks 1–13 complete, review-clean.

## UI redesign (2026-11-23 → 2026-11-24) — branch `ui-redesign`

- Worktree: `/media/mint/WDC_WD10EZRX_932G/watch/OpenWatch/.worktrees/ui-redesign`. Commits `bc86ac1`..`20ec3d4` (Tasks 1–5), then `docs: UI redesign verification and handoff` (Task 6). Nothing merged to `main` for this redesign.
- Shipped: real two-step write controls (Today Control → Confirm → exact frame); unified zinc/blue tokens, contrast fixes, ≥12px interactive type, ≥40px targets and reachable reduced-transparency/high-contrast toggles; responsive shell (no fixed 1040×720, sidebar wide / bottom nav <700px, 360dp×600 floor, shared pane header); designed empty/error/loading states, sync phases with last-sync time and failure detail, axis baseline + newest-24 chart window and export Copy; eight colourised SVG icons and one easing family with reduced-motion cross-fades.
- Verified: `cargo test --workspace` — **115 passed / 0 failed / 1 ignored** (live-hardware test), run headless with `SLINT_BACKEND=testing`; the `AppWindow::new()` test confirms headless window construction, so Task 5's SVG/modal codegen now compiles. Prior controller `cargo fmt --all -- --check` clean. Source anti-slop scan clean (no purple/gradient/glow/emoji/bounce/overshoot; 8 icons wired and colourised; one easing family).
- Headless render limitation: Slint 1.19's testing backend has no built-in framebuffer→PNG capture API. Producing a screenshot would need product/test code plus a direct image-encoding dependency (only transitive `image`/`png` exist) — out of scope for this docs-and-verification task. No PNG was produced; checks are source/state based.
- Still open (see `docs/improvement-backlog.md`): blocking refresh freeze; "Sync failed — retry" elided at 360dp; runtime clipboard/modal/SVG visual checks; Android CI/device verification of the redesign. `cargo clippy --workspace -- -D warnings` was not run in Task 6.

## Current state

- Tasks 1–13 complete: workspace/CI, FE EA codec, decoders + confidence registry, command builders, SQLite store, BLE transport + device key, connection manager, Slint desktop UI (`crates/ui` library + thin desktop/android hosts), data views/actions, AI BYOK explain, legacy/decoders + export, Android packaging + DeviceLink wiring, live-verify harness.
- Task 14 (continuous improvement loop) in progress: backlog refreshed from the SDD ledger, first cycle applied to the quiet-hours doc accuracy item. Backlog and this handoff were seeded on `main` (commits `00d7afec`, `4f66e5d`) and carried into the worktree.
- Full deferred/open ledger: `docs/improvement-backlog.md`. Authoritative rulings: `.superpowers/sdd/2026-11-22-openwatch/progress.md`.

## Commands

From the worktree root `/media/mint/WDC_WD10EZRX_932G/watch/OpenWatch/.worktrees/openwatch-impl`:

- Format check: `cargo fmt --all -- --check`
- Tests: `cargo test --workspace`
- Lint: `cargo clippy --workspace -- -D warnings`
- UI-only tests: `cargo test -p openwatch-ui`
- Live harness (ignored by default, env-gated): `cargo test -p openwatch-core --test verify -- --ignored`
- Commit (Task 14): `git add docs && git commit -m "docs: improvement backlog and first cycle"`

## Known CI-only / device-only gaps (cannot be verified from this host)

- macOS/Windows CI runners: 3-OS matrix first exercised on push; not runnable locally.
- Android typecheck / Gradle / AAB: no NDK, Skia timeout, JDK-21 D8 failure locally; JNI code typechecked offline only.
- JNI BLE runtime-permission handshake: device-only; permission request before btleplug adapter creation unverified on hardware.
- Android AAB install + R8 keep rules + BLE subscribe: device-only.
- macOS UUID device key: CoreBluetooth per-host UUID; advertised-name match unverified on real macOS.
- Task 13 live sweep: pending hardware — `hci0` was DOWN and watch not advertising at last check; harness is `#[ignore]` + env-gated.

## Python live sweep — cwd `/media/mint/WDC_WD10EZRX_932G/watch`, python `/tmp/opencode/venv/bin/python`

- `hciconfig hci0` → `UP RUNNING`; else `sudo -n rfkill unblock bluetooth && sudo -n hciconfig hci0 up`
- Presence (8 s): `timeout 30 /tmp/opencode/venv/bin/python -c 'import asyncio; from bleak import BleakScanner; from fireboltt_client import ADDR; print(ADDR.lower() in [a.address.lower() for a in asyncio.run(BleakScanner.discover(timeout=8))])'`
- Sweep: `/tmp/opencode/venv/bin/python fireboltt_sweep.py` → `fireboltt_sweep_<ts>.json`
- Weather responder: `/tmp/opencode/venv/bin/python fireboltt_weatherd.py 90 --cond sunny --temp 28 --city DELHI`

## Resume after interruption

1. Read `.superpowers/sdd/2026-11-22-openwatch/progress.md` (ledger) and `docs/improvement-backlog.md`.
2. Work in the worktree on `openwatch-impl`; do not commit to `main`.
3. Pick the top `Open` row, implement → test → update backlog + changelog → commit.
4. At branch finish, merge `openwatch-impl` into `main` so the doc-only commits coexist (different files, no expected conflict).
