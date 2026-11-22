# Contributing

OpenWatch is licensed GPL-3.0-or-later; contributing means your changes are licensed the same way.

## Before opening a pull request

```sh
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

CI runs the same commands on Ubuntu, macOS and Windows.

## Ground rules

- `crates/core` stays free of GUI and platform dependencies; frontends stay thin.
- Decoders are pure functions with offline fixtures.
- Tag every protocol capability `L` (live-verified), `S` (source-derived) or `U` (unverified).
  Never present `S` or `U` as verified.
- No telemetry, no accounts and no cloud dependency for local features.

The [design document](docs/superpowers/specs/2026-11-22-openwatch-design.md) records the decisions
that should not be re-litigated; read it before changing behaviour.
