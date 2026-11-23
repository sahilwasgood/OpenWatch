//! Test target for the live hardware verification harness.
//!
//! The harness lives in `live/verify.rs` and is `#[ignore]`d, so a plain
//! `cargo test` never scans for or writes to the watch. This file holds only
//! offline guards; it is the `verify` integration-test target.

#[path = "live/verify.rs"]
mod live;

/// Prove the hardware harness cannot run under a normal `cargo test`.
///
/// `#[ignore]` is the primary gate. The env-var check inside the harness is a
/// second, deliberate opt-in so `cargo test -- --ignored` alone still will not
/// drive hardware. Both are asserted from source so this guard stays offline.
#[test]
fn live_hardware_harness_is_ignored_by_default() {
    let source = include_str!("live/verify.rs");
    assert!(
        source.contains("#[ignore"),
        "the live harness must carry #[ignore] so cargo test never touches hardware"
    );
    assert!(
        source.contains("OPENWATCH_LIVE_VERIFY"),
        "the live harness must require an explicit OPENWATCH_LIVE_VERIFY=1 opt-in"
    );
}
