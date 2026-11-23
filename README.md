# OpenWatch

A local-first smartwatch companion and protocol workbench for the Fire-Boltt 039, with the
Noise NSW-434 scaffolded for later work.

**No account. No cloud lock-in. No telemetry.** Your watch readings and raw frames are persisted
to a local SQLite file — under `openwatch/` in your user data directory on desktop, and in the
app's files directory on Android. History, export and delete all operate on that local file;
there is no sync service. Every state-changing action requires an explicit action from you, and
the optional AI interpretation layer is off unless you opt in and supply your own key (only the
one selected frame and your note are sent, and the reviewed payload is the exact body transmitted).

## Supported platforms

| Platform | Status |
| --- | --- |
| Linux | supported |
| macOS | supported |
| Windows | supported |
| Android | supported (packaging in progress) |

One Rust workspace builds all four targets.

## Workspace layout

- `crates/core` — protocol framing, codecs, decoders, model and local store. No GUI or platform
  dependencies.
- `crates/ui` — shared Slint UI: state bridge, live link, Today/Device/Workbench/Data surfaces.
- `crates/desktop` — Slint host for Linux, macOS and Windows.
- `crates/android` — Slint host for Android.

## User interface

The four Slint panes (Today, Device, Workbench, Data) share one restrained zinc/blue token
language: real two-step write controls, distinct sync states with last-sync time and failure detail,
a responsive shell (sidebar on wide windows, bottom navigation below 700px, 360dp minimum), designed
empty/error states, a Copy action for exports, eight colourised SVG icons and one motion-easing
family. Interactive labels are ≥12px, touch targets ≥40px, and reduced-transparency / high-contrast
toggles live in Data settings. Verification: `cargo test --workspace` — 115 passed / 1 ignored
(live-hardware test); `cargo fmt --all -- --check` clean. No rendered screenshot exists yet.
See [the improvement backlog](docs/improvement-backlog.md) and [handoff](docs/HANDOFF.md) for open items.

## Build

Requires a stable Rust toolchain (edition 2021).

```sh
cargo build --workspace
cargo test --workspace
```

Run the desktop host with `cargo run -p openwatch-desktop`. Continuous integration runs
formatting, clippy and tests on Ubuntu, macOS and Windows.

## Documentation

- [Design document](docs/superpowers/specs/2026-11-22-openwatch-design.md)
- [Research summary](docs/research/2026-11-22-research-summary.md)

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
