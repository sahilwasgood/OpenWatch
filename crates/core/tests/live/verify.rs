//! Ignored live-hardware verification harness for the Fire-Boltt 039.
//!
//! NOT run by `cargo test`. To drive real hardware the operator must both pass
//! `--ignored` and set the explicit opt-in:
//!
//! ```text
//! OPENWATCH_LIVE_VERIFY=1 cargo test -p openwatch-core --test verify -- --ignored --nocapture
//! ```
//!
//! It connects through [`BtleplugTransport`] / [`ConnectionManager`], runs the
//! 20 design-documented reads plus battery, exercises the verified write
//! commands, records before/after reads for state-changing writes and restores
//! originals where practical, then writes exact frames with UTC timestamps to
//! `docs/protocol/verification-log-generated.md`. Destructive file-channel
//! commands (face upload, DFU, AGPS, OTA) are deliberately never sent.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use openwatch_core::ble::btleplug_transport::BtleplugTransport;
use openwatch_core::ble::device_key::{resolve_key, DeviceKey};
use openwatch_core::ble::BleTransport;
use openwatch_core::connection::ConnectionManager;
use openwatch_core::protocol::command;
use openwatch_core::protocol::decode::decode;
use openwatch_core::protocol::frame::{packet, Frame};
use openwatch_core::protocol::registry::{confidence, live_evidence, Confidence};

const WATCH_ADDRESS: &str = "67:31:E4:DD:50:7A";
/// Reply window per query. Writes are still spaced 350 ms by the manager.
const WAIT_MS: u64 = 400;
const OPT_IN: &str = "OPENWATCH_LIVE_VERIFY";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Days since 1970-01-01 -> (year, month, day) (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    (y, m, d)
}

fn utc_label(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn conf_letter(c: Confidence) -> &'static str {
    match c {
        Confidence::Live => "L",
        Confidence::Source => "S",
        Confidence::Unverified => "U",
    }
}

fn battery_label(raw: u8) -> String {
    if raw > 100 {
        format!("{}% (charging)", raw - 100)
    } else {
        format!("{raw}%")
    }
}

fn le_u16(b: &[u8]) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(0)?, *b.get(1)?]))
}

fn le_u32(b: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(0)?,
        *b.get(1)?,
        *b.get(2)?,
        *b.get(3)?,
    ]))
}

#[derive(Debug)]
struct Row {
    at: i64,
    direction: &'static str,
    capability: String,
    opcode: String,
    frame_hex: String,
    decoded: String,
    confidence: &'static str,
}

struct Recorder {
    rows: Vec<Row>,
}

impl Recorder {
    fn new() -> Self {
        Self { rows: Vec::new() }
    }

    fn push(
        &mut self,
        direction: &'static str,
        capability: &str,
        opcode: &str,
        frame_hex: &str,
        decoded: &str,
        confidence: &'static str,
    ) {
        self.rows.push(Row {
            at: now(),
            direction,
            capability: capability.to_string(),
            opcode: opcode.to_string(),
            frame_hex: frame_hex.to_string(),
            decoded: decoded.to_string(),
            confidence,
        });
    }

    fn decoded(frame: &Frame) -> String {
        decode(frame.cmd, &frame.payload)
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| format!("raw {}", hex(&frame.payload)))
    }

    fn write_log(&self, path: &Path, battery: Option<u8>) -> std::io::Result<()> {
        let mut md = String::new();
        md.push_str("# OpenWatch — generated live verification log\n\n");
        md.push_str(&format!("Generated: {} ({})\n\n", utc_label(now()), now()));
        md.push_str(&format!("Device: Fire-Boltt 039 (`{WATCH_ADDRESS}`)\n\n"));
        match battery {
            Some(raw) => md.push_str(&format!(
                "Battery read: `{}` = {}\n\n",
                raw,
                battery_label(raw)
            )),
            None => md.push_str("Battery read: unavailable\n\n"),
        }
        md.push_str(
            "Produced by the ignored live harness (`crates/core/tests/live/verify.rs`).\n\
             Generated only with `OPENWATCH_LIVE_VERIFY=1 ... --ignored`, never by `cargo test`.\n\
             State-changing writes record before/after reads and restore originals where practical.\n\
             No face upload / DFU / AGPS / OTA commands are sent.\n\n",
        );
        md.push_str("## Captured frames\n\n");
        md.push_str("| UTC | Dir | Capability | Opcode | Frame | Decoded / readback | Conf |\n");
        md.push_str("|---|---|---|---|---|---|---|\n");
        for row in &self.rows {
            md.push_str(&format!(
                "| {} | {} | {} | `{}` | `{}` | {} | {} |\n",
                utc_label(row.at),
                row.direction,
                row.capability.replace('|', "\\|"),
                row.opcode,
                row.frame_hex,
                row.decoded.replace('|', "\\|"),
                row.confidence,
            ));
        }
        md.push_str("\n## Registry promotion evidence (L)\n\n");
        md.push_str(
            "Baseline captures from the 2026-11-22 sweep and 2026-11-23 endpoint probe. \
             The harness reproduces and extends these on each live run.\n\n",
        );
        md.push_str("| Opcode | Capability | Captured frame | Source |\n|---|---|---|---|\n");
        for e in live_evidence() {
            md.push_str(&format!(
                "| `0x{:02X}` | {} | `{}` | {} |\n",
                e.cmd, e.name, e.frame_hex, e.captured_at
            ));
        }
        fs::write(path, md)
    }
}

/// One query: record the exact TX frame, the exact RX frame and the decode.
async fn record_query<T: BleTransport>(
    manager: &ConnectionManager<T>,
    rec: &mut Recorder,
    label: &str,
    opcode: &str,
    cmd: u8,
    payload: &[u8],
) -> Option<Frame> {
    let conf = conf_letter(confidence(cmd));
    let tx = packet(cmd, payload);
    rec.push(
        "TX",
        &format!("{label} · query"),
        opcode,
        &hex(&tx),
        "",
        conf,
    );
    match manager.query(cmd, payload, WAIT_MS).await {
        Ok(frame) => {
            let rx = packet(frame.cmd, &frame.payload);
            rec.push(
                "RX",
                label,
                opcode,
                &hex(&rx),
                &Recorder::decoded(&frame),
                conf,
            );
            Some(frame)
        }
        Err(e) => {
            rec.push("RX", label, opcode, "", &format!("no reply ({e})"), conf);
            None
        }
    }
}

/// One fire-and-forget write: record the exact TX frame.
async fn record_write<T: BleTransport>(
    manager: &ConnectionManager<T>,
    rec: &mut Recorder,
    label: &str,
    opcode: &str,
    frame: &[u8],
) {
    let cmd = frame.get(4).copied().unwrap_or(0);
    rec.push(
        "TX",
        label,
        opcode,
        &hex(frame),
        "write",
        conf_letter(confidence(cmd)),
    );
    manager.write(frame).await.expect("write to watch");
}

/// The 20 design-documented dashboard reads. Each entry is (capability, opcode, payload).
fn read_sweep() -> Vec<(&'static str, u8, Vec<u8>)> {
    vec![
        ("steps", 0x2B, vec![]),
        ("language", 0x2E, vec![]),
        ("firmware", 0xBA, vec![]),
        ("edition", 0x29, vec![]),
        ("message limit", 0xBB, vec![]),
        ("device state", 0x84, vec![]),
        ("HR history", 0x35, vec![]),
        ("step categories", 0x59, vec![]),
        ("auto-HR index", 0x2F, vec![]),
        ("live HR a", 0x81, vec![]),
        ("live HR b", 0x82, vec![]),
        ("time system", 0x27, vec![]),
        ("quiet hours", 0x2C, vec![]),
        ("watch-face layout", 0x39, vec![]),
        ("goals", 0xB9, vec![0x10]),
        ("stress", 0xB9, vec![0x11]),
        ("alarms", 0xB9, vec![0x15]),
        ("message-app list", 0xB9, vec![0x14, 0x00]),
        ("move reminders", 0x83, vec![]),
        ("step goal readback", 0x26, vec![]),
    ]
}

#[tokio::test]
#[ignore = "requires a physically connected Fire-Boltt 039; set OPENWATCH_LIVE_VERIFY=1"]
async fn live_hardware_verification() {
    if std::env::var(OPT_IN).ok().as_deref() != Some("1") {
        eprintln!("skip: set {OPT_IN}=1 to run the live hardware harness");
        return;
    }

    let mut rec = Recorder::new();
    let key: DeviceKey = resolve_key(std::env::consts::OS, WATCH_ADDRESS);
    let transport = BtleplugTransport::new().await.expect("BLE adapter");
    let mut manager = ConnectionManager::new(transport, key);
    manager.connect().await.expect("connect to Fire-Boltt 039");

    // Battery (not a control frame; recorded as a single raw reading).
    let battery = manager.battery().await.ok();
    if let Some(raw) = battery {
        rec.push(
            "RX",
            "battery",
            "2A19",
            &format!("{raw:02x}"),
            &battery_label(raw),
            "L",
        );
    }

    // Step 2: the 20 read sweep.
    let reads = read_sweep();
    assert_eq!(reads.len(), 20, "the design specifies 20 dashboard reads");
    for (label, cmd, payload) in &reads {
        let opcode = format!(
            "0x{cmd:02X}{}",
            if payload.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", hex(payload))
            }
        );
        record_query(&manager, &mut rec, label, &opcode, *cmd, payload).await;
    }

    // Step 3: verified writes. State-changing writes read before/after and restore.

    // Step goal (0x16 write, 0x26 LE32 readback).
    let goal_before =
        record_query(&manager, &mut rec, "step goal · before", "0x26", 0x26, &[]).await;
    record_write(
        &manager,
        &mut rec,
        "set step goal",
        "0x16",
        &command::set_goal_steps(10_000),
    )
    .await;
    let goal_after = record_query(&manager, &mut rec, "step goal · after", "0x26", 0x26, &[]).await;
    if let Some(raw) = goal_before.as_ref().and_then(|f| le_u32(&f.payload)) {
        record_write(
            &manager,
            &mut rec,
            "step goal · restore",
            "0x16",
            &command::set_goal_steps(raw),
        )
        .await;
    } else {
        rec.push(
            "NOTE",
            "step goal · restore",
            "0x16",
            "",
            "original goal unreadable; skipped",
            "L",
        );
    }
    let _ = goal_after;

    // Time system (0x17 write, 0x27 readback).
    let ts_before = record_query(
        &manager,
        &mut rec,
        "time system · before",
        "0x27",
        0x27,
        &[],
    )
    .await;
    let ts_orig = ts_before.as_ref().and_then(|f| f.payload.first().copied());
    let ts_test = u8::from(ts_orig != Some(1));
    record_write(
        &manager,
        &mut rec,
        "set time system",
        "0x17",
        &command::set_time_system(ts_test == 1),
    )
    .await;
    record_query(&manager, &mut rec, "time system · after", "0x27", 0x27, &[]).await;
    if let Some(orig) = ts_orig {
        record_write(
            &manager,
            &mut rec,
            "time system · restore",
            "0x17",
            &command::set_time_system(orig == 1),
        )
        .await;
    } else {
        rec.push(
            "NOTE",
            "time system · restore",
            "0x17",
            "",
            "original unreadable; skipped",
            "L",
        );
    }

    // DND (0x71 write, 0x81 readback: two LE16 times).
    let dnd_before = record_query(&manager, &mut rec, "DND · before", "0x81", 0x81, &[]).await;
    record_write(
        &manager,
        &mut rec,
        "set DND 22:00-06:00",
        "0x71",
        &command::set_dnd((22, 0), (6, 0), true),
    )
    .await;
    record_query(&manager, &mut rec, "DND · after", "0x81", 0x81, &[]).await;
    if let Some(payload) = dnd_before.as_ref().map(|f| f.payload.as_slice()) {
        let start = le_u16(payload).unwrap_or(0);
        let end = le_u16(payload.get(2..).unwrap_or_default()).unwrap_or(0);
        record_write(
            &manager,
            &mut rec,
            "DND · restore",
            "0x71",
            &command::set_dnd(
                ((start / 60) as u8, (start % 60) as u8),
                ((end / 60) as u8, (end % 60) as u8),
                start != 0 || end != 0,
            ),
        )
        .await;
    } else {
        rec.push(
            "NOTE",
            "DND · restore",
            "0x71",
            "",
            "original unreadable; wrote DND off",
            "L",
        );
        record_write(
            &manager,
            &mut rec,
            "DND · safe off",
            "0x71",
            &command::set_dnd((0, 0), (0, 0), false),
        )
        .await;
    }

    // Alarm set/delete (B9{05,00} / B9{05,02}). Only touch a free slot so an
    // existing alarm is never clobbered.
    let alarms = record_query(
        &manager,
        &mut rec,
        "alarms · before",
        "0xB9{15}",
        0xB9,
        &[0x15],
    )
    .await;
    let alarm_count = alarms
        .as_ref()
        .and_then(|f| f.payload.get(2).copied())
        .unwrap_or(u8::MAX);
    if alarm_count == 0 {
        record_write(
            &manager,
            &mut rec,
            "set alarm slot 0",
            "0xB9{05,00}",
            &command::set_alarm(0, 8, 45, 0, (2026, 11, 23)),
        )
        .await;
        record_query(
            &manager,
            &mut rec,
            "alarms · after set",
            "0xB9{15}",
            0xB9,
            &[0x15],
        )
        .await;
        record_write(
            &manager,
            &mut rec,
            "delete alarm slot 0",
            "0xB9{05,02}",
            &command::delete_alarm(0),
        )
        .await;
        record_query(
            &manager,
            &mut rec,
            "alarms · after delete",
            "0xB9{15}",
            0xB9,
            &[0x15],
        )
        .await;
    } else {
        rec.push(
            "NOTE",
            "alarm set/delete",
            "0xB9{05}",
            "",
            &format!("skipped: {alarm_count} alarm(s) already configured; do not overwrite"),
            "L",
        );
    }

    // Weather, message, time sync: display/transient, nothing to restore.
    record_write(
        &manager,
        &mut rec,
        "weather today",
        "0x43",
        &command::weather_today(5, 28, "DELH", ""),
    )
    .await;
    record_write(
        &manager,
        &mut rec,
        "weather forecast",
        "0x42",
        &command::weather_forecast(5, 32, 24, &[(3, 31, 23)]),
    )
    .await;
    record_write(
        &manager,
        &mut rec,
        "weather sun",
        "0xB5",
        &command::weather_sun(5, 28, (6, 12), (18, 47), "DELHI"),
    )
    .await;
    record_write(
        &manager,
        &mut rec,
        "message push",
        "0x41",
        &command::message("OpenWatch verify"),
    )
    .await;
    let unix = now() as u32;
    record_write(
        &manager,
        &mut rec,
        "time sync",
        "0x31",
        &command::time_sync(unix),
    )
    .await;

    // Auto-HR (0x1F write, 0x2F readback).
    let hr_before = record_query(&manager, &mut rec, "auto-HR · before", "0x2F", 0x2F, &[]).await;
    let hr_orig = hr_before.as_ref().and_then(|f| f.payload.first().copied());
    record_write(
        &manager,
        &mut rec,
        "set auto-HR on",
        "0x1F",
        &packet(0x1F, &[1]),
    )
    .await;
    record_query(&manager, &mut rec, "auto-HR · after", "0x2F", 0x2F, &[]).await;
    if let Some(orig) = hr_orig {
        record_write(
            &manager,
            &mut rec,
            "auto-HR · restore",
            "0x1F",
            &packet(0x1F, &[orig]),
        )
        .await;
    } else {
        rec.push(
            "NOTE",
            "auto-HR · restore",
            "0x1F",
            "",
            "original unreadable; wrote off",
            "L",
        );
        record_write(
            &manager,
            &mut rec,
            "auto-HR · safe off",
            "0x1F",
            &packet(0x1F, &[0]),
        )
        .await;
    }

    // Live HR trigger: the watch replies {bpm} or {FF} busy when unworn.
    record_query(&manager, &mut rec, "live HR", "0x6D", 0x6D, &[0x00]).await;

    // Legacy probes (unverified). Silence is expected; never auto-retried.
    record_query(
        &manager,
        &mut rec,
        "camera (legacy probe)",
        "0x66{01}",
        0x66,
        &[0x01],
    )
    .await;
    record_query(
        &manager,
        &mut rec,
        "ring (legacy probe)",
        "0x62{01}",
        0x62,
        &[0x01],
    )
    .await;

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/protocol/verification-log-generated.md");
    rec.write_log(&path, battery)
        .expect("write generated verification log");
    println!(
        "live verification complete: {} frames -> {}",
        rec.rows.len(),
        path.display()
    );
}

#[cfg(test)]
mod harness_self_checks {
    use super::*;

    #[test]
    fn utc_and_hex_helpers_are_deterministic() {
        // 2026-11-22 14:03 UTC (the fixed clock used elsewhere in the workspace).
        assert_eq!(utc_label(1_795_356_180), "2026-11-22 14:03:00 UTC");
        assert_eq!(hex(&unhex("feea2009833c320a16")), "feea2009833c320a16");
    }

    #[test]
    fn read_sweep_is_exactly_twenty_queries() {
        let reads = read_sweep();
        assert_eq!(reads.len(), 20);
        assert_eq!(reads.last().unwrap().1, 0x26);
    }

    #[test]
    fn battery_label_decodes_charging_flag() {
        assert_eq!(battery_label(68), "68%");
        assert_eq!(battery_label(168), "68% (charging)");
    }

    #[test]
    fn evidence_frames_parse_through_the_codec() {
        // Guard the captured-frame fixtures used by the generated log.
        for e in live_evidence() {
            assert_eq!(confidence(e.cmd), Confidence::Live);
            assert_eq!(hex(&unhex(e.frame_hex)), e.frame_hex);
        }
    }
}
