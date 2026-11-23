# Changelog

All notable changes to this project are recorded here.

## [Unreleased]

### Changed

- Corrected design-doc confidence claim: quiet hours `0x2C` is `L` for the read only; the verified write path is `0x71` (DND). Recorded as the first continuous-improvement cycle in `docs/improvement-backlog.md`.

### Added

- Cargo workspace with `openwatch-core`, `openwatch-desktop` and `openwatch-android`.
- CI matrix on Ubuntu, macOS and Windows running formatting, clippy and tests.
- Repository documentation and the GPL-3.0-or-later license.
