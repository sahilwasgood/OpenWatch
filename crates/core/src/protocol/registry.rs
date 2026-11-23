//! Confidence registry: opcode -> evidence level.

/// Evidence level for a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Live-verified on real hardware.
    Live,
    /// Derived from a trusted source, not yet live-verified.
    Source,
    /// Unverified.
    Unverified,
}

/// Confidence for a top-level opcode.
///
/// `0xB9` carries subcommands with mixed confidence; the fine-grained view is
/// in [`all_opcodes`], while this function reports the verified read default.
pub fn confidence(cmd: u8) -> Confidence {
    match cmd {
        0x2B | 0x2E | 0xBA | 0x29 | 0xBB | 0x84 | 0x35 | 0x59 | 0x2F | 0x81 | 0x82 | 0x27
        | 0x2C | 0x39 | 0x16 | 0x17 | 0x31 | 0x41 | 0x6D | 0x83 | 0xB9 | 0x26 | 0x71 | 0x1F => {
            Confidence::Live
        }
        0x43 | 0x42 | 0xB5 => Confidence::Source,
        _ => Confidence::Unverified,
    }
}

/// All registered opcodes with their names and confidence.
pub fn all_opcodes() -> Vec<(u8, &'static str, Confidence)> {
    use Confidence::*;
    vec![
        (0x2B, "steps", Live),
        (0x2E, "language", Live),
        (0xBA, "firmware", Live),
        (0x29, "edition", Live),
        (0xBB, "message limit", Live),
        (0x84, "device state", Live),
        (0x35, "HR history", Live),
        (0x59, "step categories", Live),
        (0x2F, "auto-HR index", Live),
        (0x81, "live HR a", Live),
        (0x82, "live HR b", Live),
        (0x27, "time system", Live),
        (0x2C, "quiet hours", Live),
        (0x39, "watch-face layout", Live),
        (0x26, "read step goal", Live),
        (0xB9, "goals (B9 10)", Live),
        (0xB9, "stress (B9 11)", Live),
        (0xB9, "alarms (B9 15)", Live),
        (0x16, "set step goal", Live),
        (0x17, "set time system", Live),
        (0x31, "time sync", Live),
        (0x41, "message push", Live),
        (0x6D, "start live HR", Live),
        (0x43, "weather today", Source),
        (0x42, "weather forecast", Source),
        (0xB5, "weather sun", Source),
        // Live-promoted from the 2026-11-22 sweep: each write was confirmed by a readback.
        (0x71, "set DND", Live),
        (0x1F, "set auto-HR", Live),
        (0xB9, "set/delete alarm (B9 05)", Live),
        // Live-promoted from the 2026-11-23 endpoint probe captures.
        (0x83, "move reminders", Live),
        (0xB9, "message-app list (B9 14 00)", Live),
        // Silently ignored or layout-undecoded: stay unverified, never auto-run.
        (0x66, "camera", Unverified),
        (0x62, "ring", Unverified),
        (0x85, "women's health", Unverified),
        (0xB6, "day-pack probes", Unverified),
        // shortcut: face/DFU/AGPS/OTA use the FEE6 file channel, not a control opcode;
        // 0xF6 is a sentinel so they can be listed here. Replace with real opcodes when verified.
        (0xF6, "face upload (FEE6)", Unverified),
        (0xF6, "DFU (FEE6)", Unverified),
        (0xF6, "AGPS (FEE6)", Unverified),
        (0xF6, "OTA (FEE6)", Unverified),
    ]
}

/// An exact captured frame backing a [`Confidence::Live`] promotion.
#[derive(Debug, Clone, Copy)]
pub struct FrameEvidence {
    pub cmd: u8,
    pub name: &'static str,
    /// Lowercase hex of the captured FE EA control frame.
    pub frame_hex: &'static str,
    /// Source evidence file / session for the capture.
    pub captured_at: &'static str,
}

/// Captures recovered from the 2026-11-22 sweep and 2026-11-23 endpoint probe.
/// The ignored live harness reproduces and extends these on real hardware and
/// writes the extended set to `docs/protocol/verification-log-generated.md`.
pub fn live_evidence() -> &'static [FrameEvidence] {
    &[
        FrameEvidence {
            cmd: 0x26,
            name: "read step goal",
            frame_hex: "feea20092610270000",
            captured_at: "2026-11-23 endpoint probe",
        },
        FrameEvidence {
            cmd: 0x83,
            name: "move reminders",
            frame_hex: "feea2009833c320a16",
            captured_at: "2026-11-23 endpoint probe",
        },
        FrameEvidence {
            cmd: 0xB9,
            name: "message-app list (B9 14 00)",
            frame_hex: "feea201bb9140013000102030405060708090a0b0c0d0e0f111f80",
            captured_at: "2026-11-23 endpoint probe",
        },
        FrameEvidence {
            cmd: 0x71,
            name: "set DND",
            frame_hex: "feea20097116000600",
            captured_at: "2026-11-22 sweep",
        },
        FrameEvidence {
            cmd: 0x1F,
            name: "set auto-HR",
            frame_hex: "feea20061f01",
            captured_at: "2026-11-22 sweep",
        },
        FrameEvidence {
            cmd: 0x16,
            name: "set step goal",
            frame_hex: "feea20091600002710",
            captured_at: "2026-11-22 sweep",
        },
        FrameEvidence {
            cmd: 0x31,
            name: "time sync",
            frame_hex: "feea200a31f889026b08",
            captured_at: "2026-11-22 sweep",
        },
        FrameEvidence {
            cmd: 0x41,
            name: "message push",
            frame_hex: "feea201641804f70656e576174636820766572696679",
            captured_at: "2026-11-22 sweep",
        },
        FrameEvidence {
            cmd: 0x6D,
            name: "live HR trigger",
            frame_hex: "feea20066d00",
            captured_at: "2026-11-22 sweep",
        },
        FrameEvidence {
            cmd: 0xB9,
            name: "set alarm (B9 05 00)",
            frame_hex: "feea200fb90500000101071eb10100",
            captured_at: "2026-11-22 sweep",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_opcodes() {
        for cmd in [
            0x2B, 0x2E, 0xBA, 0x29, 0xBB, 0x84, 0x35, 0x59, 0x2F, 0x81, 0x82, 0x27, 0x2C, 0x39,
            0x16, 0x17, 0x31, 0x41, 0x6D, 0x83, 0x26, 0x71, 0x1F,
        ] {
            assert_eq!(confidence(cmd), Confidence::Live, "cmd {cmd:02X}");
        }
    }

    #[test]
    fn source_opcodes() {
        for cmd in [0x43, 0x42, 0xB5] {
            assert_eq!(confidence(cmd), Confidence::Source, "cmd {cmd:02X}");
        }
    }

    #[test]
    fn unverified_opcodes() {
        for cmd in [0x66, 0x62, 0x85, 0xB6] {
            assert_eq!(confidence(cmd), Confidence::Unverified, "cmd {cmd:02X}");
        }
    }

    #[test]
    fn live_verified_legacy_opcodes() {
        assert_eq!(confidence(0x83), Confidence::Live);
        let ops = all_opcodes();
        assert!(ops
            .iter()
            .any(|(c, n, l)| *c == 0x83 && *l == Confidence::Live && n.contains("move reminders")));
        assert!(ops.iter().any(|(c, n, l)| *c == 0xB9
            && *l == Confidence::Live
            && n.contains("message-app list")));
        for cmd in [0x66, 0x62, 0x85, 0xB6] {
            assert!(
                ops.iter()
                    .any(|(c, _, l)| *c == cmd && *l == Confidence::Unverified),
                "cmd {cmd:02X} must stay unverified"
            );
        }
    }

    #[test]
    fn registry_covers_every_opcode() {
        let ops = all_opcodes();
        for cmd in [
            0x2B, 0x2E, 0xBA, 0x29, 0xBB, 0x84, 0x35, 0x59, 0x2F, 0x81, 0x82, 0x27, 0x2C, 0x39,
            0x16, 0x17, 0x31, 0x41, 0x6D, 0x43, 0x42, 0xB5, 0x71, 0x1F, 0x66, 0x62, 0x83, 0x85,
            0xB6, 0x26,
        ] {
            assert!(ops.iter().any(|(c, _, _)| *c == cmd), "missing {cmd:02X}");
        }
        // B9 subcommands are listed separately with their own confidence.
        assert!(ops
            .iter()
            .any(|(c, n, l)| *c == 0xB9 && *l == Confidence::Live && n.contains("goals")));
        assert!(ops
            .iter()
            .any(|(c, n, l)| *c == 0xB9 && *l == Confidence::Live && n.contains("alarm")));
        assert!(ops.iter().any(|(c, n, l)| *c == 0xB9
            && *l == Confidence::Live
            && n.contains("message-app list")));
        // File-channel features are unverified.
        assert!(ops
            .iter()
            .any(|(_, n, l)| *l == Confidence::Unverified && n.contains("face upload")));
        assert!(ops
            .iter()
            .any(|(_, n, l)| *l == Confidence::Unverified && n.contains("DFU")));
        assert!(ops
            .iter()
            .any(|(_, n, l)| *l == Confidence::Unverified && n.contains("AGPS")));
        assert!(ops
            .iter()
            .any(|(_, n, l)| *l == Confidence::Unverified && n.contains("OTA")));
    }

    #[test]
    fn live_evidence_frames_back_live_opcodes() {
        use crate::protocol::frame::parse;
        let evidence = live_evidence();
        assert!(!evidence.is_empty());
        for entry in evidence {
            assert_eq!(
                confidence(entry.cmd),
                Confidence::Live,
                "{} has a capture but is not promoted",
                entry.name
            );
            let bytes: Vec<u8> = (0..entry.frame_hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&entry.frame_hex[i..i + 2], 16).unwrap())
                .collect();
            assert!(
                parse(&bytes).is_ok(),
                "{} capture must be a valid FE EA frame",
                entry.name
            );
            assert!(
                !entry.captured_at.is_empty(),
                "{} needs a source",
                entry.name
            );
        }
        // The step-goal read query is promoted with its 2026-11-23 capture.
        assert!(evidence
            .iter()
            .any(|e| e.cmd == 0x26 && e.frame_hex == "feea20092610270000"));
    }
}
