//! Canonicalisation pre-pass that defangs the most common encoding-shaped
//! bypasses before the regex layer runs.
//!
//! Stage-1 of the redactor promotion (see eval report
//! `docs/skycot/evals/scheduled/2026-05-23/eval-redact-crate-promote-from-regex-to-real-fence.md`).
//! Closes 14 of 23 PII bypass misses identified by the
//! `encoding_bypass_corpus_test` audit: fullwidth-digit cards,
//! fullwidth `@`, URL-encoded `%XX`, HTML entity `&#NN;`/`&amp;`,
//! zero-width-space + soft-hyphen split, and base64-wrapped payloads.
//!
//! ## Offset projection
//!
//! Every transformation records a per-byte offset map back to the
//! original input. Callers run their scan over [`Canonical::text`] and
//! project each match span back via [`Canonical::original_span`] so the
//! masked output keeps the un-canonicalised text everywhere except where
//! a sentinel replaces a hit.

use std::ops::Range;
use unicode_normalization::UnicodeNormalization;

/// A canonicalised view of an input string plus a byte-level offset map
/// back to the original. `offsets` has length `text.len() + 1` so a
/// half-open range `[start, end)` in `text` projects to
/// `[offsets[start], offsets[end])` in the original input.
#[derive(Debug, Clone)]
pub(crate) struct Canonical {
    /// Canonicalised text. Run the regex scan over this string.
    pub text: String,
    /// One entry per canonical byte position, plus a trailing entry at
    /// `text.len()` that maps to the original input length.
    pub offsets: Vec<usize>,
}

impl Canonical {
    /// Project a canonical span back to the original input.
    pub fn original_span(&self, span: Range<usize>) -> Range<usize> {
        let start = self.offsets.get(span.start).copied().unwrap_or(0);
        let end = self
            .offsets
            .get(span.end)
            .copied()
            .unwrap_or_else(|| self.offsets.last().copied().unwrap_or(0));
        start..end
    }

    /// Identity canonicalisation — used when chaining passes can short-circuit.
    fn identity(input: &str) -> Self {
        let offsets: Vec<usize> = (0..=input.len()).collect();
        Canonical {
            text: input.to_string(),
            offsets,
        }
    }
}

/// Canonicalise `input` through the Stage-1 pipeline: NFKC + ZWSP /
/// soft-hyphen strip → URL-decode → HTML-entity decode. Each pass
/// composes its offsets onto the original input so the returned map
/// stays referenced to `input`.
///
/// Base64 detect-and-rescan is NOT included here — it lives in
/// [`scan_base64_tokens`] because it requires a recursive call into the
/// redactor and so depends on `RedactionPolicy`.
pub(crate) fn canonicalize_for_scan(input: &str) -> Canonical {
    let p1 = nfkc_strip(input);
    let p2 = url_decode(&p1.text);
    let p2 = compose(p2, &p1.offsets);
    let p3 = html_entity_decode(&p2.text);
    compose(p3, &p2.offsets)
}

/// Pass 1: per-char NFKC normalisation, with zero-width-space (U+200B,
/// U+200C, U+200D, U+FEFF) and soft hyphen (U+00AD) stripped. Per-char
/// (vs whole-string) NFKC preserves the property that every canonical
/// byte derives from exactly one source char — required for the offset
/// map. Combining-mark composition that crosses char boundaries (e.g.
/// `a` + U+0301 → `á`) is deliberately NOT performed here; it stays on
/// the Stage 3 (NER) backlog.
fn nfkc_strip(input: &str) -> Canonical {
    let mut text = String::with_capacity(input.len());
    let mut offsets = Vec::with_capacity(input.len() + 1);
    let mut buf = [0u8; 4];
    for (orig_pos, c) in input.char_indices() {
        if is_invisible_strip(c) {
            continue;
        }
        for nc in std::iter::once(c).nfkc() {
            let encoded = nc.encode_utf8(&mut buf);
            for _ in 0..encoded.len() {
                offsets.push(orig_pos);
            }
            text.push_str(encoded);
        }
    }
    offsets.push(input.len());
    Canonical { text, offsets }
}

fn is_invisible_strip(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'   // soft hyphen
            | '\u{200B}' // zero-width space
            | '\u{200C}' // zero-width non-joiner
            | '\u{200D}' // zero-width joiner
            | '\u{FEFF}' // zero-width no-break space (BOM)
    )
}

/// Pass 2: decode `%XX` escapes where the decoded byte is a printable
/// ASCII char. Anything else passes through unchanged. Restricting the
/// fold to printable ASCII keeps `text` valid UTF-8 by construction —
/// we never splice a stray continuation byte into a multi-byte char.
fn url_decode(input: &str) -> Canonical {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut offsets: Vec<usize> = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            if let Some(dec) = hex_pair(bytes[i + 1], bytes[i + 2]) {
                if dec.is_ascii() && dec >= 0x20 && dec != 0x7F {
                    out.push(dec);
                    offsets.push(i);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(b);
        offsets.push(i);
        i += 1;
    }
    offsets.push(input.len());
    let text = match String::from_utf8(out) {
        Ok(s) => s,
        Err(_) => return Canonical::identity(input),
    };
    debug_assert_eq!(offsets.len(), text.len() + 1);
    Canonical { text, offsets }
}

fn hex_pair(hi: u8, lo: u8) -> Option<u8> {
    let h = (hi as char).to_digit(16)?;
    let l = (lo as char).to_digit(16)?;
    Some((h * 16 + l) as u8)
}

/// Pass 3: decode HTML decimal/hex numeric character references
/// (`&#NN;`, `&#xHH;`) and a small whitelist of named entities common
/// in PII-bearing strings: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`,
/// `&commat;`, `&period;`. Bounded scope: full HTML5 named-entity
/// support adds ~2000 entries and is unnecessary for the leak surface
/// the Stage-1 corpus covers.
fn html_entity_decode(input: &str) -> Canonical {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut offsets: Vec<usize> = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some((decoded, len)) = decode_entity_at(&bytes[i..]) {
                for &db in decoded.as_bytes() {
                    out.push(db);
                    offsets.push(i);
                }
                i += len;
                continue;
            }
        }
        out.push(bytes[i]);
        offsets.push(i);
        i += 1;
    }
    offsets.push(input.len());
    let text = match String::from_utf8(out) {
        Ok(s) => s,
        Err(_) => return Canonical::identity(input),
    };
    debug_assert_eq!(offsets.len(), text.len() + 1);
    Canonical { text, offsets }
}

fn decode_entity_at(tail: &[u8]) -> Option<(&'static str, usize)> {
    if tail.first() != Some(&b'&') {
        return None;
    }
    let end = tail.iter().take(12).position(|&b| b == b';')?;
    let body = &tail[1..end];
    let len = end + 1;
    if body.is_empty() {
        return None;
    }
    if body[0] == b'#' {
        let cp = parse_numeric_entity(&body[1..])?;
        return numeric_to_ascii(cp).map(|s| (s, len));
    }
    let named = match body {
        b"amp" => Some(AMP),
        b"lt" => Some(LT),
        b"gt" => Some(GT),
        b"quot" => Some(QUOT),
        b"apos" => Some(APOS),
        b"commat" => Some(AT),
        b"period" => Some(PERIOD),
        _ => None,
    };
    named.map(|s| (s, len))
}

const AMP: &str = "&";
const LT: &str = "<";
const GT: &str = ">";
const QUOT: &str = "\"";
const APOS: &str = "'";
const AT: &str = "@";
const PERIOD: &str = ".";

fn parse_numeric_entity(body: &[u8]) -> Option<u32> {
    if body.is_empty() {
        return None;
    }
    if body[0] == b'x' || body[0] == b'X' {
        let s = std::str::from_utf8(&body[1..]).ok()?;
        u32::from_str_radix(s, 16).ok()
    } else {
        let s = std::str::from_utf8(body).ok()?;
        s.parse::<u32>().ok()
    }
}

fn numeric_to_ascii(cp: u32) -> Option<&'static str> {
    let c = char::from_u32(cp)?;
    if !c.is_ascii() {
        return None;
    }
    Some(match c {
        '@' => AT,
        '.' => PERIOD,
        '&' => AMP,
        '<' => LT,
        '>' => GT,
        '"' => QUOT,
        '\'' => APOS,
        '-' => HYPHEN,
        '_' => UNDERSCORE,
        '/' => SLASH,
        '\\' => BACKSLASH,
        ':' => COLON,
        ';' => SEMI,
        '=' => EQ,
        '+' => PLUS,
        '?' => QMARK,
        '#' => HASH,
        '!' => BANG,
        '$' => DOLLAR,
        '%' => PERCENT,
        '(' => LPAREN,
        ')' => RPAREN,
        ' ' => SPACE,
        _ => return None,
    })
}

const HYPHEN: &str = "-";
const UNDERSCORE: &str = "_";
const SLASH: &str = "/";
const BACKSLASH: &str = "\\";
const COLON: &str = ":";
const SEMI: &str = ";";
const EQ: &str = "=";
const PLUS: &str = "+";
const QMARK: &str = "?";
const HASH: &str = "#";
const BANG: &str = "!";
const DOLLAR: &str = "$";
const PERCENT: &str = "%";
const LPAREN: &str = "(";
const RPAREN: &str = ")";
const SPACE: &str = " ";

/// Compose `outer` (whose `offsets` reference `inner_offsets`'s text
/// space) onto the original byte space tracked by `inner_offsets`.
fn compose(outer: Canonical, inner_offsets: &[usize]) -> Canonical {
    let max = inner_offsets.last().copied().unwrap_or(0);
    let offsets: Vec<usize> = outer
        .offsets
        .iter()
        .map(|&i| inner_offsets.get(i).copied().unwrap_or(max))
        .collect();
    Canonical {
        text: outer.text,
        offsets,
    }
}

/// Decode a base64 token verbatim and return the decoded text if it
/// decodes successfully AND the result is UTF-8. Returns `None` for
/// arbitrary binary blobs (which would otherwise produce noise hits).
///
/// Used by the redactor for the detect-and-rescan path: when a long
/// base64-shaped run appears in canonicalised text, we decode it and
/// run the same regex layer over the decoded bytes; any hit in the
/// decoded space causes the entire base64 token in the canonical text
/// to be masked.
pub(crate) fn decode_base64_token(token: &str) -> Option<String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    if !token.len().is_multiple_of(4) || token.len() < 12 {
        return None;
    }
    let bytes = STANDARD.decode(token).ok()?;
    String::from_utf8(bytes).ok()
}

/// True for strings that look like a single contiguous base64 token of
/// at least 12 chars — `[A-Za-z0-9+/=]` with `=` only as trailing
/// padding. The redactor only attempts base64 decode on tokens that
/// pass this gate, to avoid trying every short alphanumeric run.
pub(crate) fn looks_like_base64(token: &str) -> bool {
    if token.len() < 12 {
        return false;
    }
    let mut saw_pad = false;
    for c in token.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '+' | '/' => {
                if saw_pad {
                    return false;
                }
            }
            '=' => {
                saw_pad = true;
            }
            _ => return false,
        }
    }
    token.len().is_multiple_of(4)
}

#[cfg(test)]
#[path = "canonicalize_tests.rs"]
mod tests;
