# OpenWatch — handoff (2026-11-22)

Repo `sahilwasgood/OpenWatch` at `/media/mint/WDC_WD10EZRX_932G/watch/OpenWatch`. Branch `main` @32bde9f (doc-only); implementation on `openwatch-impl` in `.worktrees/openwatch-impl` @8f72e6a.

**Complete:** Tasks 1–7 — scaffolding/CI, FE EA codec, decoders + confidence registry, command builders, SQLite store, BLE transport + device key, connection manager; 37 tests pass, fmt/clippy clean; live writes logged in `docs/protocol/verification-log.md`.

**In progress:** Task 8 (Desktop UI shell/theme); worktree has untracked `crates/ui/` and modified `Cargo.lock`. Ledger ruling: all `.slint` files and `AppState` live in the `crates/ui` library — Android cannot link a binary crate. Tasks 9–14 remain.

**Resume:** read the SDD plan and `.superpowers/sdd/2026-11-22-openwatch/progress.md`; work in the worktree; finish 8, then 9–14 (Task 14 is this loop; backlog seeded at `docs/improvement-backlog.md`). At branch finish merge `openwatch-impl` into `main` so doc commits 9bf09b8/32bde9f coexist.

**Python live sweep** — cwd `/media/mint/WDC_WD10EZRX_932G/watch`, python `/tmp/opencode/venv/bin/python`:
- `hciconfig hci0` → `UP RUNNING`; else `sudo -n rfkill unblock bluetooth && sudo -n hciconfig hci0 up`
- presence (8 s): `timeout 30 /tmp/opencode/venv/bin/python -c 'import asyncio; from bleak import BleakScanner; from fireboltt_client import ADDR; print(ADDR.lower() in [a.address.lower() for a in asyncio.run(BleakScanner.discover(timeout=8))])'`
- sweep: `/tmp/opencode/venv/bin/python fireboltt_sweep.py` → `fireboltt_sweep_<ts>.json`
- weather responder: `/tmp/opencode/venv/bin/python fireboltt_weatherd.py 90 --cond sunny --temp 28 --city DELHI`
