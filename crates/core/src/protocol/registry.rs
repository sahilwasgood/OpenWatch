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
        | 0x2C | 0x39 | 0x16 | 0x17 | 0x31 | 0x41 | 0x6D | 0xB9 => Confidence::Live,
        0x43 | 0x42 | 0xB5 | 0x71 | 0x1F => Confidence::Source,
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
        (0x71, "set DND", Source),
        (0x1F, "set auto-HR", Source),
        (0xB9, "set/delete alarm (B9 05)", Source),
        (0x66, "camera", Unverified),
        (0x62, "ring", Unverified),
        (0x83, "move reminders", Unverified),
        (0x85, "women's health", Unverified),
        (0xB6, "day-pack probes", Unverified),
        (0xB9, "message-app list (B9 20 00)", Unverified),
        // shortcut: face/DFU/AGPS/OTA use the FEE6 file channel, not a control opcode;
        // 0xF6 is a sentinel so they can be listed here. Replace with real opcodes when verified.
        (0xF6, "face upload (FEE6)", Unverified),
        (0xF6, "DFU (FEE6)", Unverified),
        (0xF6, "AGPS (FEE6)", Unverified),
        (0xF6, "OTA (FEE6)", Unverified),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_opcodes() {
        for cmd in [
            0x2B, 0x2E, 0xBA, 0x29, 0xBB, 0x84, 0x35, 0x59, 0x2F, 0x81, 0x82, 0x27, 0x2C,
            0x39, 0x16, 0x17, 0x31, 0x41, 0x6D,
        ] {
            assert_eq!(confidence(cmd), Confidence::Live, "cmd {cmd:02X}");
        }
    }

    #[test]
    fn source_opcodes() {
        for cmd in [0x43, 0x42, 0xB5, 0x71, 0x1F] {
            assert_eq!(confidence(cmd), Confidence::Source, "cmd {cmd:02X}");
        }
    }

    #[test]
    fn unverified_opcodes() {
        for cmd in [0x66, 0x62, 0x83, 0x85, 0xB6] {
            assert_eq!(confidence(cmd), Confidence::Unverified, "cmd {cmd:02X}");
        }
    }

    #[test]
    fn registry_covers_every_opcode() {
        let ops = all_opcodes();
        for cmd in [
            0x2B, 0x2E, 0xBA, 0x29, 0xBB, 0x84, 0x35, 0x59, 0x2F, 0x81, 0x82, 0x27, 0x2C,
            0x39, 0x16, 0x17, 0x31, 0x41, 0x6D, 0x43, 0x42, 0xB5, 0x71, 0x1F, 0x66, 0x62,
            0x83, 0x85, 0xB6,
        ] {
            assert!(ops.iter().any(|(c, _, _)| *c == cmd), "missing {cmd:02X}");
        }
        // B9 subcommands are listed separately with their own confidence.
        assert!(ops.iter().any(|(c, n, l)| *c == 0xB9 && *l == Confidence::Live && n.contains("goals")));
        assert!(ops.iter().any(|(c, n, l)| *c == 0xB9 && *l == Confidence::Source && n.contains("alarm")));
        // File-channel features are unverified.
        assert!(ops.iter().any(|(_, n, l)| *l == Confidence::Unverified && n.contains("face upload")));
        assert!(ops.iter().any(|(_, n, l)| *l == Confidence::Unverified && n.contains("DFU")));
        assert!(ops.iter().any(|(_, n, l)| *l == Confidence::Unverified && n.contains("AGPS")));
        assert!(ops.iter().any(|(_, n, l)| *l == Confidence::Unverified && n.contains("OTA")));
    }
}
