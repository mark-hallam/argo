//! Tail-buffer + stream-pump helpers for [`super::ProcessRegistry`].
//!
//! Split out of `process_registry.rs` to keep both files under the
//! 500-LOC ceiling. The buffer and the OS-signal helpers are
//! self-contained; they do not borrow any registry state.

use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

/// Bounded ring buffer that keeps the most recent `cap` bytes plus
/// a counter of dropped bytes for "[truncated …]" messages.
#[derive(Debug)]
pub struct TailBuffer {
    buf: Vec<u8>,
    cap: usize,
    dropped: u64,
}

impl TailBuffer {
    /// Create a new buffer with capacity `cap` bytes.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap.min(8 * 1024)),
            cap,
            dropped: 0,
        }
    }

    /// Append `chunk`; oldest bytes are evicted to keep the buffer
    /// at most `cap` bytes.
    pub fn push(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }
        if chunk.len() >= self.cap {
            let extra = self.buf.len() as u64 + (chunk.len() - self.cap) as u64;
            self.dropped = self.dropped.saturating_add(extra);
            self.buf.clear();
            self.buf.extend_from_slice(&chunk[chunk.len() - self.cap..]);
            return;
        }
        let total = self.buf.len() + chunk.len();
        if total > self.cap {
            let drop = total - self.cap;
            self.buf.drain(..drop);
            self.dropped = self.dropped.saturating_add(drop as u64);
        }
        self.buf.extend_from_slice(chunk);
    }

    /// Return the last `up_to` bytes as UTF-8 (lossy) plus a
    /// truncation marker if any bytes were dropped.
    #[must_use]
    pub fn render(&self, up_to: usize) -> String {
        let slice = if self.buf.len() > up_to {
            &self.buf[self.buf.len() - up_to..]
        } else {
            &self.buf[..]
        };
        let mut out = String::from_utf8_lossy(slice).into_owned();
        let dropped_total = self.dropped + (self.buf.len() - slice.len()) as u64;
        if dropped_total > 0 {
            out.push_str(&format!(
                "\n... [truncated {dropped_total} bytes from this stream]"
            ));
        }
        out
    }

    /// Total bytes dropped due to capacity pressure.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

pub(super) fn spawn_pump<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    mut reader: R,
    sink: Arc<Mutex<TailBuffer>>,
) {
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => sink.lock().await.push(&buf[..n]),
            }
        }
    });
}

#[cfg(unix)]
pub(super) fn send_sigterm(pid: u32) {
    // SAFETY: libc::kill is safe to call from any thread; pid 0 /
    // negative are intentionally not delivered here (tokio gives
    // the child's positive pid).
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
}

#[cfg(not(unix))]
pub(super) fn send_sigterm(_pid: u32) {
    // Non-Unix: fall through to tokio's `kill()` (SIGKILL-equivalent).
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_buffer_evicts_oldest_bytes() {
        let mut tb = TailBuffer::new(8);
        tb.push(b"abcdefghij"); // 10 bytes, cap 8
        let s = tb.render(64);
        assert!(s.starts_with("cdefghij"), "got: {s:?}");
        assert!(s.contains("[truncated"));
        assert_eq!(tb.dropped(), 2);
    }

    #[test]
    fn tail_buffer_handles_appends_under_cap() {
        let mut tb = TailBuffer::new(16);
        tb.push(b"abc");
        tb.push(b"def");
        let s = tb.render(64);
        assert_eq!(s, "abcdef");
        assert_eq!(tb.dropped(), 0);
    }
}
