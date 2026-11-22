# OpenWatch

A local-first smartwatch companion and protocol workbench for the Fire-Boltt 039, with the
Noise NSW-434 scaffolded for later work.

**No account. No cloud lock-in. No telemetry.** Your watch data stays on your machine, every
state-changing action requires an explicit action from you, and the optional AI interpretation
layer is off unless you opt in and supply your own key.

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
- `crates/desktop` — Slint host for Linux, macOS and Windows.
- `crates/android` — Slint host for Android.

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
