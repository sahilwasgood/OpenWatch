//! FE EA frame codec for the watch protocol.

use thiserror::Error;

const MAGIC: [u8; 2] = [0xFE, 0xEA];
const HEADER_LEN: usize = 5;
const LENGTH_BASE: usize = 0x20;

/// Errors returned when parsing an invalid FE EA frame.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("frame too short")]
    TooShort,
    #[error("bad magic")]
    BadMagic,
    #[error("bad length")]
    BadLength,
}

/// A decoded FE EA frame.
#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    pub cmd: u8,
    pub payload: Vec<u8>,
}

/// Build a control frame: `FE EA [0x20+(n>>8)] [n] cmd payload…`.
pub fn packet(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let total = HEADER_LEN + payload.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&MAGIC);
    out.push((LENGTH_BASE + (total >> 8)) as u8);
    out.push(total as u8);
    out.push(cmd);
    out.extend_from_slice(payload);
    out
}

/// Parse a control frame, validating the magic and length fields.
pub fn parse(data: &[u8]) -> Result<Frame, ProtocolError> {
    if data.len() < MAGIC.len() || data[..MAGIC.len()] != MAGIC {
        return Err(ProtocolError::BadMagic);
    }
    if data.len() < HEADER_LEN {
        return Err(ProtocolError::TooShort);
    }
    if data[3] as usize != data.len() {
        return Err(ProtocolError::BadLength);
    }
    Ok(Frame {
        cmd: data[4],
        payload: data[HEADER_LEN..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_empty_payload() {
        assert_eq!(packet(0x53, &[]), vec![0xFE, 0xEA, 0x20, 0x05, 0x53]);
    }

    #[test]
    fn packet_single_byte_payload() {
        assert_eq!(
            packet(0x66, &[0x01]),
            vec![0xFE, 0xEA, 0x20, 0x06, 0x66, 0x01]
        );
    }

    #[test]
    fn packet_large_payload_header() {
        let bytes = packet(0x00, &[0xAB; 300]);
        assert_eq!(&bytes[..4], &[0xFE, 0xEA, 0x21, 0x31]);
    }

    #[test]
    fn parse_round_trip() {
        assert_eq!(
            parse(&packet(0xB9, &[0x15, 0x04])).unwrap(),
            Frame {
                cmd: 0xB9,
                payload: vec![0x15, 0x04],
            }
        );
    }

    #[test]
    fn parse_bad_magic() {
        assert_eq!(parse(&[0x00, 0x01]), Err(ProtocolError::BadMagic));
    }

    #[test]
    fn parse_bad_length() {
        assert_eq!(
            parse(&[0xFE, 0xEA, 0x20, 0x09, 0x53]),
            Err(ProtocolError::BadLength)
        );
    }
}
