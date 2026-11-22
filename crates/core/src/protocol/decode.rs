//! Pure decoders for watch reply payloads, plus reply selection.

use crate::protocol::frame::Frame;

/// A decoded value. Multi-field replies are returned as raw [`Value::Bytes`]
/// so the caller can read the documented field offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Number(u64),
    Text(String),
    Bytes(Vec<u8>),
    Flag(bool),
    Time(String),
}

/// Decode a known reply payload. Unknown opcodes return `None`.
pub fn decode(cmd: u8, payload: &[u8]) -> Option<Value> {
    let be_u32 = |at: usize| {
        let b = payload.get(at..at + 4)?;
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64)
    };
    let le_u16 = |at: usize| {
        let b = payload.get(at..at + 2)?;
        Some(u16::from_le_bytes([b[0], b[1]]) as u64)
    };

    match cmd {
        0x2E => Some(Value::Text(
            match *payload.first()? {
                0 => "chinese",
                1 => "international",
                _ => "unknown",
            }
            .into(),
        )),
        0x27 => Some(Value::Text(
            match *payload.first()? {
                0 => "12h",
                1 => "24h",
                _ => "unknown",
            }
            .into(),
        )),
        0xBA => {
            if payload.is_empty() {
                return None;
            }
            Some(Value::Text(
                payload
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<_>>()
                    .join("."),
            ))
        }
        0x2B => Some(Value::Number(be_u32(1)?)),
        0xBB => Some(Value::Number(le_u16(1)?)),
        0x39 => {
            // width, height, thumb width, thumb height (BE u16), then "ORIGINAL".
            if payload.len() < 16 {
                return None;
            }
            Some(Value::Bytes(payload.to_vec()))
        }
        0xB9 => match *payload.first()? {
            // goals: [0x10][?] then [type u8][LE u32] pairs from byte 2
            0x10 => (payload.len() >= 7).then(|| Value::Bytes(payload.to_vec())),
            // alarms: count at byte 2, 8-byte records from byte 3
            0x15 => {
                let count = *payload.get(2)? as usize;
                (payload.len() >= 3 + 8 * count).then(|| Value::Bytes(payload.to_vec()))
            }
            // stress: state/timestamp/hourly
            0x11 => (payload.len() >= 2).then(|| Value::Bytes(payload.to_vec())),
            _ => None,
        },
        // Multi-field replies without a dedicated Value variant: preserve raw bytes.
        0x29 | 0x84 | 0x35 | 0x59 | 0x2F | 0x81 | 0x82 | 0x2C => {
            Some(Value::Bytes(payload.to_vec()))
        }
        _ => None,
    }
}

/// Return the first frame matching `cmd`, ignoring spontaneous frames
/// such as `0xF9` keepalives.
pub fn select_reply(frames: &[Frame], cmd: u8) -> Option<Frame> {
    let found = frames.iter().find(|frame| frame.cmd == cmd)?;
    Some(Frame {
        cmd: found.cmd,
        payload: found.payload.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_mask() {
        assert_eq!(
            decode(0x2E, &[1]),
            Some(Value::Text("international".into()))
        );
    }

    #[test]
    fn firmware_version() {
        assert_eq!(
            decode(0xBA, &[1, 0, 40]),
            Some(Value::Text("1.0.40".into()))
        );
    }

    #[test]
    fn steps_be_u32() {
        // [prefix][BE u32] -> 0x00001DF3
        assert_eq!(
            decode(0x2B, &[0, 0, 0, 0x1D, 0xF3]),
            Some(Value::Number(0x00001DF3))
        );
    }

    #[test]
    fn message_limit_le_u16() {
        // [0x0B][LE u16] -> 240
        assert_eq!(decode(0xBB, &[0x0B, 0xF0, 0x00]), Some(Value::Number(240)));
    }

    #[test]
    fn time_system() {
        assert_eq!(decode(0x27, &[1]), Some(Value::Text("24h".into())));
    }

    #[test]
    fn face_layout_46_bytes() {
        // width, height, thumb width, thumb height (BE u16) then "ORIGINAL".
        let mut layout = vec![0u8; 46];
        layout[0..2].copy_from_slice(&240u16.to_be_bytes());
        layout[2..4].copy_from_slice(&284u16.to_be_bytes());
        layout[4..6].copy_from_slice(&140u16.to_be_bytes());
        layout[6..8].copy_from_slice(&163u16.to_be_bytes());
        layout[8..16].copy_from_slice(b"ORIGINAL");
        match decode(0x39, &layout) {
            Some(Value::Bytes(b)) => {
                assert_eq!(u16::from_be_bytes([b[0], b[1]]), 240);
                assert_eq!(u16::from_be_bytes([b[2], b[3]]), 284);
                assert_eq!(u16::from_be_bytes([b[4], b[5]]), 140);
                assert_eq!(u16::from_be_bytes([b[6], b[7]]), 163);
                assert_eq!(&b[8..16], b"ORIGINAL");
            }
            other => panic!("expected Bytes, got {other:?}"),
        }
    }

    #[test]
    fn b9_goals_are_type_le_u32_pairs_from_byte_two() {
        let payload = [
            0x10, 0x00, 0x01, 0x10, 0x27, 0x00, 0x00, 0x02, 0x40, 0x1F, 0x00, 0x00,
        ];
        match decode(0xB9, &payload) {
            Some(Value::Bytes(b)) => {
                assert_eq!(b, payload);
                assert_eq!(u32::from_le_bytes([b[3], b[4], b[5], b[6]]), 10000);
                assert_eq!(u32::from_le_bytes([b[8], b[9], b[10], b[11]]), 8000);
            }
            other => panic!("expected Bytes, got {other:?}"),
        }
    }

    #[test]
    fn b9_alarms_count_and_records() {
        let mut payload = vec![0x15, 0x04, 0x01];
        payload.extend_from_slice(&[0x00, 0x01, 0x00, 0x07, 0x1E, 0xBA, 0x13, 0x00]);
        match decode(0xB9, &payload) {
            Some(Value::Bytes(b)) => {
                assert_eq!(b[2], 1, "alarm count at byte 2");
                assert_eq!(b.len(), 11, "count + one 8-byte record");
            }
            other => panic!("expected Bytes, got {other:?}"),
        }
    }

    #[test]
    fn b9_stress_payload() {
        let payload = [0x11, 0x01, 0x02, 0x03, 0x04];
        assert_eq!(decode(0xB9, &payload), Some(Value::Bytes(payload.to_vec())));
    }

    #[test]
    fn select_reply_skips_keepalive_and_returns_first_match() {
        let keepalive = Frame {
            cmd: 0xF9,
            payload: vec![],
        };
        let unrelated = Frame {
            cmd: 0x2B,
            payload: vec![0],
        };
        let later = Frame {
            cmd: 0x2E,
            payload: vec![0],
        };
        let frames = [
            keepalive,
            unrelated,
            Frame {
                cmd: 0x2E,
                payload: vec![1],
            },
            later,
        ];
        assert_eq!(
            select_reply(&frames, 0x2E),
            Some(Frame {
                cmd: 0x2E,
                payload: vec![1]
            })
        );
    }

    #[test]
    fn select_reply_none_when_no_match() {
        let frames = [Frame {
            cmd: 0xF9,
            payload: vec![],
        }];
        assert_eq!(select_reply(&frames, 0x2E), None);
    }

    #[test]
    fn unknown_cmd_decodes_to_none() {
        assert_eq!(decode(0x53, &[0x00]), None);
    }
}
