//! Minimal HTML → plaintext converter for email fallbacks.
//!
//! Strips tags, decodes a small set of named/numeric entities, collapses
//! whitespace runs, and wraps body lines at 72 chars. The goal is a
//! readable text/plain alternative for MIME multipart sends, not a faithful
//! HTML renderer.

/// Convert an HTML fragment to a plaintext fallback wrapped at 72 columns.
///
/// # Examples
///
/// ```
/// use argo_email_templates::plaintext::html_to_plaintext;
/// let txt = html_to_plaintext("<p>Hello <strong>world</strong>&mdash;welcome.</p>");
/// assert!(txt.contains("Hello world\u{2014}welcome."));
/// ```
pub fn html_to_plaintext(html: &str) -> String {
    let stripped = strip_tags(html);
    let decoded = decode_entities(&stripped);
    let collapsed = collapse_whitespace(&decoded);
    wrap_lines(&collapsed, 72)
}

fn strip_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();
    for ch in input.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let name = tag_buf.trim_start_matches('/').to_ascii_lowercase();
                let name = name.split_whitespace().next().unwrap_or("");
                if matches!(name, "br" | "p" | "tr" | "div" | "li" | "h1" | "h2" | "h3" | "h4") {
                    out.push('\n');
                }
            }
            c if in_tag => tag_buf.push(c),
            c => out.push(c),
        }
    }
    out
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        let mut name = String::new();
        let mut consumed = 0usize;
        let mut closed = false;
        while let Some(&nc) = iter.peek() {
            consumed += 1;
            if nc == ';' {
                iter.next();
                closed = true;
                break;
            }
            if consumed > 8 {
                break;
            }
            name.push(nc);
            iter.next();
        }
        if !closed {
            out.push('&');
            out.push_str(&name);
            continue;
        }
        let decoded = if let Some(rest) = name.strip_prefix('#') {
            let n = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
                u32::from_str_radix(hex, 16).ok()
            } else {
                rest.parse::<u32>().ok()
            };
            n.and_then(char::from_u32)
        } else {
            match name.as_str() {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{00A0}'),
                "copy" => Some('©'),
                "reg" => Some('®'),
                "middot" => Some('·'),
                "mdash" => Some('\u{2014}'),
                "ndash" => Some('\u{2013}'),
                "rsquo" => Some('\u{2019}'),
                "lsquo" => Some('\u{2018}'),
                "rdquo" => Some('\u{201D}'),
                "ldquo" => Some('\u{201C}'),
                "rarr" => Some('\u{2192}'),
                _ => None,
            }
        };
        if let Some(c) = decoded {
            out.push(c);
        } else {
            out.push('&');
            out.push_str(&name);
            out.push(';');
        }
    }
    out
}

fn collapse_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut blank_run = 0usize;
    for raw_line in input.split('\n') {
        let mut line = String::with_capacity(raw_line.len());
        let mut prev_space = false;
        for ch in raw_line.chars() {
            if ch.is_whitespace() {
                if !prev_space && !line.is_empty() {
                    line.push(' ');
                }
                prev_space = true;
            } else {
                line.push(ch);
                prev_space = false;
            }
        }
        let trimmed = line.trim_end().to_string();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 1 && !out.is_empty() {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(&trimmed);
            out.push('\n');
        }
    }
    out.trim_end_matches('\n').to_string() + "\n"
}

fn wrap_lines(input: &str, width: usize) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for line in input.split('\n') {
        if line.chars().count() <= width {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            let needed = if current.is_empty() {
                word.chars().count()
            } else {
                current.chars().count() + 1 + word.chars().count()
            };
            if needed > width && !current.is_empty() {
                out.push_str(&current);
                out.push('\n');
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            out.push_str(&current);
            out.push('\n');
        }
    }
    out.trim_end_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_decodes_entities() {
        let html = "<p>Hello <strong>world</strong>&mdash;welcome.</p>";
        let txt = html_to_plaintext(html);
        assert!(!txt.contains('<'), "tags must be stripped: {txt}");
        assert!(txt.contains("Hello world\u{2014}welcome."), "got: {txt}");
    }

    #[test]
    fn wraps_long_lines_at_72_chars() {
        let html = format!("<p>{}</p>", "word ".repeat(60));
        let txt = html_to_plaintext(&html);
        for line in txt.lines() {
            assert!(
                line.chars().count() <= 72,
                "line {} chars: {line:?}",
                line.chars().count()
            );
        }
    }

    #[test]
    fn block_tags_introduce_newlines() {
        let html = "<p>one</p><p>two</p>";
        let txt = html_to_plaintext(html);
        assert!(txt.contains("one"));
        assert!(txt.contains("two"));
        assert!(txt.contains('\n'));
    }

    #[test]
    fn numeric_entities_decode() {
        let txt = html_to_plaintext("&#8212;&#x2192;");
        assert!(txt.contains('\u{2014}'));
        assert!(txt.contains('\u{2192}'));
    }
}
