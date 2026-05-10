//! Tiny base64 decoder with no external crate. OpenAI returns images
//! as `b64_json`; we decode them inline so the connector stays
//! dependency-light.

const ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Decode standard-alphabet base64 (whitespace ignored, padding required).
pub fn decode(input: &str) -> Result<Vec<u8>, String> {
    let mut lookup = [0xFFu8; 256];
    for (idx, byte) in ALPHABET.iter().enumerate() {
        lookup[*byte as usize] = idx as u8;
    }
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }
    if !cleaned.len().is_multiple_of(4) {
        return Err(format!("base64 length {} not a multiple of 4", cleaned.len()));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks(4) {
        let mut buf = [0u8; 4];
        let mut pad = 0;
        for (i, byte) in chunk.iter().enumerate() {
            if *byte == b'=' {
                pad += 1;
                buf[i] = 0;
                continue;
            }
            let v = lookup[*byte as usize];
            if v == 0xFF {
                return Err(format!("invalid base64 byte 0x{byte:02x}"));
            }
            buf[i] = v;
        }
        let triple = (u32::from(buf[0]) << 18)
            | (u32::from(buf[1]) << 12)
            | (u32::from(buf[2]) << 6)
            | u32::from(buf[3]);
        out.push(((triple >> 16) & 0xFF) as u8);
        if pad < 2 {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if pad < 1 {
            out.push((triple & 0xFF) as u8);
        }
    }
    Ok(out)
}

/// Encode bytes as standard-alphabet base64. Used in tests to round-trip
/// fixtures through [`decode`].
#[cfg(test)]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, b) in chunk.iter().enumerate() {
            buf[i] = *b;
        }
        let n = chunk.len();
        let triple =
            (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if n >= 2 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if n >= 3 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_standard_alphabet() {
        let raw = b"PNG\x00\x01\x02";
        assert_eq!(decode(&encode(raw)).unwrap(), raw);
    }

    #[test]
    fn rejects_non_multiple_of_four() {
        let err = decode("ABC").unwrap_err();
        assert!(err.contains("not a multiple of 4"));
    }

    #[test]
    fn rejects_invalid_byte() {
        let err = decode("AAA*").unwrap_err();
        assert!(err.contains("invalid base64 byte"));
    }
}
