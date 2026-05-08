//! Tracing-subscriber layer that pattern-scrubs known secret shapes
//! (bearer tokens, API keys, passwords, JWTs) from log output before
//! any writer sees them. Defense-in-depth safety net for the
//! workspace-wide tracing chain — sites that take secrets via typed
//! config should still avoid logging them; this layer is the floor.
//!
//! Wired into the subscriber chain by
//! [`crate::tracing_setup::init`]. Opt-out for local debug only via
//! `ARGO_TRACING_SCRUB=0`.
//!
//! ## Implementation note
//!
//! `tracing_subscriber` 0.3 events are immutable — a layer cannot
//! mutate field values and forward a "rewritten" event to other
//! layers. The brief's literal "intercept fields, then forward"
//! framing is therefore satisfied here by having
//! `SecretsScrubberLayer` itself be the write target: when enabled,
//! it owns the formatting + write role for events and the chain
//! omits `fmt::layer()`. The opt-out path keeps `fmt::layer()` as
//! before. Either way, no writer sees an unscrubbed value.

use std::borrow::Cow;
use std::env;
use std::fmt::Write as _;
use std::io::{self, Write as IoWrite};
use std::sync::{Mutex, OnceLock};

use regex::Regex;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Env var that disables scrubbing when set to `0`. Any other value
/// (or absence) keeps scrubbing on.
pub const ENV_OPT_OUT: &str = "ARGO_TRACING_SCRUB";

/// Resolve scrubber-enabled state from a raw env value. Pure
/// function — testable without touching process env.
pub fn enabled_from_env(raw: Option<&str>) -> bool {
    !matches!(raw, Some("0"))
}

/// Resolve scrubber-enabled state from the live process env.
pub fn enabled() -> bool {
    enabled_from_env(env::var(ENV_OPT_OUT).ok().as_deref())
}

/// Scrub known secret shapes from `s`. Returns `Cow::Borrowed(s)`
/// when no pattern matched (zero alloc on the common path). Operates
/// on free-form text — for tracing fields with a sensitive *name*,
/// use [`scrub_field`] so name-driven redaction applies even when
/// the value alone is not pattern-recognizable.
pub fn scrub(s: &str) -> Cow<'_, str> {
    let mut current: Cow<'_, str> = Cow::Borrowed(s);
    for (re, replacement) in patterns() {
        if re.is_match(&current) {
            let replaced = re.replace_all(&current, *replacement).into_owned();
            current = Cow::Owned(replaced);
        }
    }
    current
}

/// Field-aware scrub: when the tracing field's *name* itself
/// indicates a secret (`password`, `api_key`, `bearer`, …), redact
/// the value unconditionally — the bare value is not pattern-
/// recognizable on its own (e.g. `sk_live_abc...` doesn't carry the
/// `api_key=` prefix the inline regex needs). Otherwise fall through
/// to [`scrub`] for inline-pattern detection.
pub fn scrub_field<'a>(name: &str, value: &'a str) -> Cow<'a, str> {
    let lower = name.to_ascii_lowercase();
    if matches!(lower.as_str(), "password" | "passwd" | "pwd") {
        return Cow::Borrowed("<scrubbed:password>");
    }
    if lower.contains("apikey")
        || lower.contains("api_key")
        || lower.contains("api-key")
    {
        return Cow::Borrowed("<scrubbed:apikey>");
    }
    if lower == "bearer"
        || lower.contains("access_token")
        || lower.contains("access-token")
        || lower.contains("refresh_token")
        || lower.contains("refresh-token")
        || lower == "secret"
        || lower.ends_with("_secret")
        || lower.ends_with("-secret")
    {
        return Cow::Borrowed("<scrubbed:secret>");
    }
    scrub(value)
}

/// Compiled regex set, one-time-init via `OnceLock`. Patterns that
/// fail to compile (none should) are silently dropped to keep this
/// function infallible; the scrubber is a safety net, not a
/// correctness gate.
fn patterns() -> &'static [(Regex, &'static str)] {
    static CELL: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Order matters: JWT before generic high-entropy because a
        // JWT's three base64 segments would otherwise be matched as
        // an api_key value first.
        let raw: &[(&str, &str)] = &[
            (
                r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
                "<scrubbed:jwt>",
            ),
            (
                r"(?i)Bearer\s+[A-Za-z0-9\-_\.=]+",
                "Bearer <scrubbed:bearer>",
            ),
            (
                r#"(?i)(api[_-]?key|apikey)(["':\s=]+)"?[A-Za-z0-9\-_]{16,}"?"#,
                "$1$2<scrubbed:apikey>",
            ),
            (
                r#"(?i)(password|passwd|pwd)(["':\s=]+)"?[^\s"',}]+"?"#,
                "$1$2<scrubbed:password>",
            ),
            (
                r#"(?i)(secret|access[_-]?token|refresh[_-]?token)(["':\s=]+)"?[A-Za-z0-9_\-]{32,}"?"#,
                "$1$2<scrubbed:secret>",
            ),
        ];
        raw.iter()
            .filter_map(|(p, r)| Regex::new(p).ok().map(|re| (re, *r)))
            .collect()
    })
}

/// Tracing layer that scrubs each event's string fields and writes
/// the formatted line to an owned writer. When this layer is in the
/// chain, `fmt::layer()` is intentionally omitted to avoid duplicate
/// output (see module docs).
pub struct SecretsScrubberLayer {
    enabled: bool,
    writer: Mutex<Box<dyn IoWrite + Send>>,
}

impl SecretsScrubberLayer {
    /// Build a scrubber writing to `stderr`, with enabled state from
    /// `ARGO_TRACING_SCRUB`.
    pub fn new() -> Self {
        Self::with_writer(Box::new(io::stderr()))
    }

    /// Build a scrubber with a caller-provided writer. Used by the
    /// init path's standard wiring and by tests with a buffer.
    pub fn with_writer(writer: Box<dyn IoWrite + Send>) -> Self {
        Self {
            enabled: enabled(),
            writer: Mutex::new(writer),
        }
    }

    /// Test-only constructor letting the test pin the enabled state
    /// without racing other tests on `env::set_var`.
    #[cfg(test)]
    pub(crate) fn with_writer_and_enabled(
        writer: Box<dyn IoWrite + Send>,
        enabled: bool,
    ) -> Self {
        Self {
            enabled,
            writer: Mutex::new(writer),
        }
    }
}

impl Default for SecretsScrubberLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for SecretsScrubberLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = ScrubVisitor::new(self.enabled);
        event.record(&mut visitor);

        let mut line = String::with_capacity(128);
        let _ = write!(&mut line, "{} {}", metadata.level(), metadata.target());
        if !visitor.message.is_empty() {
            let _ = write!(&mut line, " {}", visitor.message);
        }
        for (k, v) in &visitor.fields {
            let _ = write!(&mut line, " {k}={v}");
        }
        line.push('\n');

        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(line.as_bytes());
        }
    }
}

/// Field visitor that captures each recorded value, scrubs string-
/// shaped values, and stashes the result. The `message` field is
/// kept separately for ordered output (level target message k=v ...).
struct ScrubVisitor {
    enabled: bool,
    message: String,
    fields: Vec<(String, String)>,
}

impl ScrubVisitor {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            message: String::new(),
            fields: Vec::new(),
        }
    }

    fn store(&mut self, name: &str, value: String) {
        let scrubbed = if self.enabled {
            scrub_field(name, &value).into_owned()
        } else {
            value
        };
        if name == "message" {
            self.message = scrubbed;
        } else {
            self.fields.push((name.to_string(), scrubbed));
        }
    }
}

impl Visit for ScrubVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.store(field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.store(field.name(), format!("{value:?}"));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.store(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.store(field.name(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.store(field.name(), value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.store(field.name(), value.to_string());
    }
}
