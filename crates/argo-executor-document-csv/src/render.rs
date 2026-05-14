//! Standalone RFC 4180 helper — string slices in, CSV bytes out.
//!
//! For the typed-schema pipeline with sandboxing, coercion, and a
//! sha256-stamped write report, use [`crate::DocumentCsv::write`].
//! `render_rfc4180` is the low-level fallback for handler-level CSV
//! emitters that already have pre-built rows of strings (the audit,
//! approval, and SOC2 exports). It exists to give those callers one
//! place to converge on instead of each maintaining its own quoting
//! routine.
//!
//! Spec:
//!   * CRLF line terminator.
//!   * Fields containing `,`, `"`, `CR`, or `LF` are wrapped in `"`.
//!   * Embedded `"` is escaped by doubling.
//!   * Empty `header` slice skips the header row.

const NEEDS_QUOTE: [char; 4] = [',', '"', '\r', '\n'];

/// Render `header` and `rows` to RFC 4180-conforming CSV bytes. See
/// the module docs for the spec. Pure function — no I/O, no
/// allocation beyond the returned buffer.
pub fn render_rfc4180<H, R>(header: &[H], rows: &[Vec<R>]) -> Vec<u8>
where
    H: AsRef<str>,
    R: AsRef<str>,
{
    let mut out: Vec<u8> = Vec::new();
    if !header.is_empty() {
        push_record(&mut out, header.iter().map(AsRef::as_ref));
    }
    for row in rows {
        push_record(&mut out, row.iter().map(AsRef::as_ref));
    }
    out
}

fn push_record<'a>(out: &mut Vec<u8>, fields: impl Iterator<Item = &'a str>) {
    let mut first = true;
    for f in fields {
        if !first {
            out.push(b',');
        }
        push_field(out, f);
        first = false;
    }
    out.extend_from_slice(b"\r\n");
}

fn push_field(out: &mut Vec<u8>, f: &str) {
    if f.chars().any(|c| NEEDS_QUOTE.contains(&c)) {
        out.push(b'"');
        for c in f.chars() {
            if c == '"' {
                out.extend_from_slice(b"\"\"");
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
        }
        out.push(b'"');
    } else {
        out.extend_from_slice(f.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_field_with_comma() {
        let body = render_rfc4180(&["a", "b"], &[vec!["x,y".to_string(), "z".to_string()]]);
        assert_eq!(String::from_utf8(body).unwrap(), "a,b\r\n\"x,y\",z\r\n");
    }

    #[test]
    fn doubles_internal_quote() {
        let body = render_rfc4180(&["q"], &[vec![r#"She said "hi""#.to_string()]]);
        assert_eq!(
            String::from_utf8(body).unwrap(),
            "q\r\n\"She said \"\"hi\"\"\"\r\n"
        );
    }

    #[test]
    fn quotes_field_with_newline() {
        let body = render_rfc4180(&["body"], &[vec!["line1\nline2".to_string()]]);
        assert_eq!(String::from_utf8(body).unwrap(), "body\r\n\"line1\nline2\"\r\n");
    }

    #[test]
    fn omits_header_when_empty() {
        let header: &[&str] = &[];
        let body = render_rfc4180(header, &[vec!["x".to_string(), "y".to_string()]]);
        assert_eq!(String::from_utf8(body).unwrap(), "x,y\r\n");
    }

    #[test]
    fn empty_rows_produces_header_only() {
        let body: Vec<u8> = render_rfc4180::<&str, &str>(&["a", "b"], &[]);
        assert_eq!(String::from_utf8(body).unwrap(), "a,b\r\n");
    }

    #[test]
    fn accepts_string_header_slice() {
        let header: Vec<String> = vec!["a".into(), "b".into()];
        let body = render_rfc4180(&header, &[vec!["1".to_string(), "2".to_string()]]);
        assert_eq!(String::from_utf8(body).unwrap(), "a,b\r\n1,2\r\n");
    }

    #[test]
    fn quotes_carriage_return() {
        let body = render_rfc4180(&["x"], &[vec!["a\rb".to_string()]]);
        assert_eq!(String::from_utf8(body).unwrap(), "x\r\n\"a\rb\"\r\n");
    }
}
