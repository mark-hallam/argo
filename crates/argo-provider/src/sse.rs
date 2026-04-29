//! Shared Server-Sent Events parser used by every SSE-speaking transport.
//!
//! Each transport handles its own dispatch (OpenAI uses `[DONE]`, Anthropic
//! looks at the `type` field of the JSON payload, …) — this module only
//! takes care of byte-level event extraction.
//!
//! ```
//! use argo_provider::SseBuffer;
//! let mut b = SseBuffer::new();
//! b.extend("data: {\"a\":1}\n\n");
//! assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
//! ```

/// Buffer of partial SSE bytes; pulls one full event per [`Self::take_event`]
/// call.
///
/// SSE events are terminated by a blank line (`\n\n` or `\r\n\r\n`). Each
/// event is a sequence of `field: value` lines. We honour `data:` only;
/// `event:`, `id:`, and `retry:` are ignored at this layer (consumers
/// dispatch on the JSON payload itself).
#[derive(Debug, Default)]
pub struct SseBuffer {
    buf: String,
}

impl SseBuffer {
    /// Empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append more bytes from the wire.
    pub fn extend(&mut self, chunk: &str) {
        self.buf.push_str(chunk);
    }

    /// Pop the next complete event from the buffer, if any. Returns the
    /// concatenated `data:` payload (multi-line `data:` joins with `\n`).
    pub fn take_event(&mut self) -> Option<String> {
        let term = self
            .buf
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| self.buf.find("\n\n").map(|i| (i, 2)))?;
        let (end, sep_len) = term;
        let event_text: String = self.buf[..end].to_string();
        self.buf.drain(..end + sep_len);
        Some(extract_data_lines(&event_text))
    }
}

fn extract_data_lines(event_text: &str) -> String {
    let mut data = String::new();
    for line in event_text.split('\n') {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_takes_one_event_at_a_time() {
        let mut b = SseBuffer::new();
        b.extend("data: {\"a\":1}\n\ndata: {\"b\":2}\n\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
        assert_eq!(b.take_event().as_deref(), Some("{\"b\":2}"));
        assert!(b.take_event().is_none());
    }

    #[test]
    fn buffer_handles_split_chunks() {
        let mut b = SseBuffer::new();
        b.extend("data: {\"a\":");
        assert!(b.take_event().is_none());
        b.extend("1}\n\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn buffer_handles_crlf_line_endings() {
        let mut b = SseBuffer::new();
        b.extend("data: {\"a\":1}\r\n\r\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn buffer_joins_multi_line_data() {
        let mut b = SseBuffer::new();
        b.extend("data: line1\ndata: line2\n\n");
        assert_eq!(b.take_event().as_deref(), Some("line1\nline2"));
    }

    #[test]
    fn buffer_ignores_non_data_fields() {
        let mut b = SseBuffer::new();
        b.extend("event: message_start\ndata: {\"x\":1}\nid: 1\n\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"x\":1}"));
    }

    #[test]
    fn buffer_returns_empty_string_for_event_without_data() {
        let mut b = SseBuffer::new();
        b.extend("event: ping\n\n");
        assert_eq!(b.take_event().as_deref(), Some(""));
    }
}
