//! OSC-52 clipboard write — paste a string into the terminal's
//! system clipboard via the
//! `ESC ] 52 ; c ; <base64> BEL` escape sequence (RFC: xterm
//! ctlseqs). Works in iTerm2, kitty, alacritty, recent xterm,
//! and tmux with `set-clipboard on`. Silently no-ops in
//! terminals that swallow the sequence.
//!
//! Self-contained — no `base64` dep needed.

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(B64[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64[((n >> 6) & 0x3F) as usize] as char);
        out.push(B64[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(B64[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(B64[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

/// Write the OSC-52 sequence to stdout. Best-effort — swallowed
/// terminals just don't see anything.
pub(crate) fn write_to_clipboard(text: &str) {
    use std::io::Write;
    let encoded = encode(text.as_bytes());
    // Use BEL (\x07) terminator for broadest compatibility; some
    // terminals also accept ESC \ but BEL is what the original
    // xterm spec specifies.
    let seq = format!("\x1b]52;c;{encoded}\x07");
    let _ = std::io::stdout().write_all(seq.as_bytes());
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_short_inputs() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"a"), "YQ==");
        assert_eq!(encode(b"ab"), "YWI=");
        assert_eq!(encode(b"abc"), "YWJj");
        // Standard RFC 4648 vector
        assert_eq!(encode(b"Man"), "TWFu");
    }

    #[test]
    fn encode_longer_input() {
        let input = b"hello, world!";
        assert_eq!(encode(input), "aGVsbG8sIHdvcmxkIQ==");
    }
}
