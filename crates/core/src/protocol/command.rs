//! Write-side command builders. Every function returns a complete FE EA frame.

use crate::protocol::frame::packet;

const MAX_TEXT_BYTES: usize = 240;

/// Cut to the byte limit without splitting a UTF-8 code point.
fn capped_utf8(text: &str) -> &[u8] {
    let mut end = MAX_TEXT_BYTES.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text.as_bytes()[..end]
}

/// Four UTF-16BE code units, space-padded.
fn utf16be_4(text: &str) -> [u8; 8] {
    let mut out = [0x00, 0x20, 0x00, 0x20, 0x00, 0x20, 0x00, 0x20];
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut len = units.len().min(4);
    // A cut after a high surrogate would strand its low half; drop it instead.
    if len > 0 && (0xD800..=0xDBFF).contains(&units[len - 1]) {
        len -= 1;
    }
    for (i, unit) in units[..len].iter().enumerate() {
        out[i * 2..i * 2 + 2].copy_from_slice(&unit.to_be_bytes());
    }
    out
}

/// Push a text message (`0x80` + at most 240 UTF-8 bytes).
pub fn message(text: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + MAX_TEXT_BYTES);
    payload.push(0x80);
    payload.extend_from_slice(capped_utf8(text));
    packet(0x41, &payload)
}

/// Sync watch time: LE unix seconds plus the fixed `0x08` suffix.
pub fn time_sync(unix: u32) -> Vec<u8> {
    let mut payload = unix.to_le_bytes().to_vec();
    payload.push(0x08);
    packet(0x31, &payload)
}

/// Set the daily step goal (BE u32).
pub fn set_goal_steps(steps: u32) -> Vec<u8> {
    packet(0x16, &steps.to_be_bytes())
}

/// Set 12h (`false`) or 24h (`true`) display.
pub fn set_time_system(h24: bool) -> Vec<u8> {
    packet(0x17, &[u8::from(h24)])
}

/// Set quiet hours; disabling sends all-zero times.
pub fn set_dnd(start: (u8, u8), end: (u8, u8), enable: bool) -> Vec<u8> {
    let (start, end) = if enable {
        (start, end)
    } else {
        ((0, 0), (0, 0))
    };
    packet(0x71, &[start.0, start.1, end.0, end.1])
}

/// Create or overwrite an alarm (`B9 05 00` + 8-byte record).
pub fn set_alarm(slot: u8, hour: u8, minute: u8, repeat: u8, date: (u16, u8, u8)) -> Vec<u8> {
    let (year, month, day) = date;
    // Protocol year is 4 bits (2015..=2030); clamp so it cannot bleed into month.
    let year = year.clamp(2015, 2030);
    let packed = (((year - 2015) << 4) | (u16::from(month) & 0x0F)) as u8;
    let mut payload = vec![0x05, 0x00];
    payload.extend_from_slice(&[slot, 0x01, repeat, hour, minute, packed, day, 0x00]);
    packet(0xB9, &payload)
}

/// Remove an alarm (`B9 05 02 slot`).
pub fn delete_alarm(slot: u8) -> Vec<u8> {
    packet(0xB9, &[0x05, 0x02, slot])
}

/// Trigger a phone-driven live heart-rate measurement.
pub fn start_hr() -> Vec<u8> {
    packet(0x6D, &[0x00])
}

/// Today's weather: condition, temperature, then festival and city names.
pub fn weather_today(cond: u8, temp: i8, city: &str, festival: &str) -> Vec<u8> {
    let mut payload = vec![0x00, cond, temp as u8];
    payload.extend_from_slice(&utf16be_4(festival));
    payload.extend_from_slice(&utf16be_4(city));
    packet(0x43, &payload)
}

/// 24 bytes: today's condition/high/low plus seven day triples.
pub fn weather_forecast(cond: u8, max: i8, min: i8, days: &[(u8, i8, i8)]) -> Vec<u8> {
    let mut payload = vec![cond, max as u8, min as u8];
    for i in 0..7 {
        let (c, high, low) = days.get(i).copied().unwrap_or((7, -100, -100));
        payload.extend_from_slice(&[c, high as u8, low as u8]);
    }
    packet(0x42, &payload)
}

/// Sunrise/sunset times and a UTF-8 location string.
pub fn weather_sun(
    cond: u8,
    temp: i8,
    sunrise: (u8, u8),
    sunset: (u8, u8),
    location: &str,
) -> Vec<u8> {
    let mut payload = vec![
        0x00, cond, temp as u8, 0x00, 0x00, sunrise.0, sunrise.1, sunset.0, sunset.1,
    ];
    payload.extend_from_slice(location.as_bytes());
    packet(0xB5, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn builders_match_captured_frames() {
        assert_eq!(hex(&set_goal_steps(8000)), "feea20091600001f40");
        assert_eq!(hex(&set_time_system(true)), "feea20061701");
        assert_eq!(
            hex(&set_alarm(0, 7, 30, 0, (2026, 10, 19))),
            "feea200fb90500000100071eba1300"
        );
        assert_eq!(hex(&delete_alarm(3)), "feea2008b9050203");
        assert_eq!(hex(&start_hr()), "feea20066d00");
        assert_eq!(hex(&message("HELLO")), "feea200b418048454c4c4f");
        assert_eq!(
            hex(&weather_today(5, 28, "DELH", "")),
            "feea20184300051c002000200020002000440045004c0048"
        );
        assert_eq!(hex(&time_sync(0x6B02_89F8)), "feea200a31f889026b08");
    }

    #[test]
    fn set_dnd_encodes_times_or_zeros() {
        assert_eq!(hex(&set_dnd((22, 0), (6, 0), true)), "feea20097116000600");
        assert_eq!(hex(&set_dnd((22, 0), (6, 0), false)), "feea20097100000000");
    }

    #[test]
    fn weather_forecast_and_sun_match_captures() {
        assert_eq!(
            hex(&weather_forecast(5, 32, 24, &[(3, 31, 23)])),
            "feea201d42052018031f17079c9c079c9c079c9c079c9c079c9c079c9c"
        );
        assert_eq!(
            hex(&weather_sun(5, 28, (6, 12), (18, 47), "DELHI")),
            "feea2013b500051c0000060c122f44454c4849"
        );
    }

    #[test]
    fn message_caps_payload_by_utf8_bytes() {
        let frame = message(&"é".repeat(300));
        assert_eq!(frame[4], 0x41);
        assert_eq!(frame.len() - 5, 241, "type byte + at most 240 UTF-8 bytes");
        assert_eq!(frame[5], 0x80);
        assert_eq!(&frame[6..], "é".repeat(120).as_bytes());
    }

    #[test]
    fn message_never_splits_a_multibyte_char() {
        let frame = message(&format!("{}€", "é".repeat(119)));
        assert_eq!(frame.len() - 5, 239, "238 bytes kept, € dropped whole");
        assert!(std::str::from_utf8(&frame[6..]).is_ok());
    }

    #[test]
    fn utf16be_4_never_ends_with_a_lone_high_surrogate() {
        let enc = utf16be_4("ABC😀");
        assert_ne!(&enc[6..8], &0xD83Du16.to_be_bytes());
        assert_eq!(enc, [0x00, 0x41, 0x00, 0x42, 0x00, 0x43, 0x00, 0x20]);
    }

    #[test]
    fn set_alarm_clamps_year_without_touching_month() {
        let high = set_alarm(0, 7, 30, 0, (2031, 10, 19));
        assert_eq!(hex(&high), hex(&set_alarm(0, 7, 30, 0, (2030, 10, 19))));
        assert_eq!(high[12] & 0x0F, 10, "month nibble intact");
        assert_eq!(high[12] >> 4, 15, "year clamped to 2030");
    }
}
