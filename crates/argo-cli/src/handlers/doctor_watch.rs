//! `argo doctor watch` — continuous doctor JSON envelope output.
use argo_types::{ArgoError, ArgoResult};
use serde_json::Value;
use std::io::Write;
use std::time::Duration;

const DEFAULT_INTERVAL: Duration = Duration::from_secs(30);
const CLEAR_SCREEN: &str = "\x1b[2J\x1b[H";
const MAX_TICKS_ENV: &str = "ARGO_DOCTOR_WATCH_MAX_TICKS";

/// Run `argo doctor watch` until Ctrl-C.
pub async fn run(interval: &str, clear: bool) -> ArgoResult<()> {
    let interval = parse_interval(interval)?;
    let max_ticks = max_ticks_from_env();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    run_loop(interval, clear, max_ticks, &mut out).await
}

async fn run_loop<W: Write>(
    interval: Duration,
    clear: bool,
    max_ticks: Option<u64>,
    out: &mut W,
) -> ArgoResult<()> {
    let mut ticks = 0_u64;
    loop {
        ticks += 1;
        let payload = super::doctor::collect().await?;
        write_cycle(out, &payload, clear)?;
        if max_ticks.is_some_and(|max| ticks >= max) {
            break;
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tokio::time::sleep(interval) => {}
        }
    }
    Ok(())
}

fn write_cycle<W: Write>(out: &mut W, payload: &Value, clear: bool) -> ArgoResult<()> {
    let rendered = render_cycle(payload, clear)?;
    out.write_all(rendered.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("doctor watch write: {e}")))?;
    out.flush()
        .map_err(|e| ArgoError::Provider(format!("doctor watch flush: {e}")))
}

fn render_cycle(payload: &Value, clear: bool) -> ArgoResult<String> {
    let json = serde_json::to_string(payload)
        .map_err(|e| ArgoError::Provider(format!("doctor watch serialize: {e}")))?;
    let mut out = String::new();
    if clear {
        out.push_str(CLEAR_SCREEN);
    }
    out.push_str(&json);
    out.push('\n');
    Ok(out)
}

fn parse_interval(raw: &str) -> ArgoResult<Duration> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(DEFAULT_INTERVAL);
    }
    let duration = if let Some(ms) = raw.strip_suffix("ms") {
        Duration::from_millis(parse_positive(ms, raw)?)
    } else if let Some(secs) = raw.strip_suffix('s') {
        Duration::from_secs(parse_positive(secs, raw)?)
    } else if let Some(mins) = raw.strip_suffix('m') {
        let mins = parse_positive(mins, raw)?;
        let secs = mins.checked_mul(60).ok_or_else(|| {
            ArgoError::Provider(format!("invalid --interval '{raw}': duration too large"))
        })?;
        Duration::from_secs(secs)
    } else {
        Duration::from_secs(parse_positive(raw, raw)?)
    };
    if duration.is_zero() {
        return Err(ArgoError::Provider(
            "invalid --interval: must be greater than zero".into(),
        ));
    }
    Ok(duration)
}

fn parse_positive(value: &str, raw: &str) -> ArgoResult<u64> {
    value.trim().parse::<u64>().map_err(|_| {
        ArgoError::Provider(format!(
            "invalid --interval '{raw}' (expected integer seconds, ms, s, or m)"
        ))
    })
}

fn max_ticks_from_env() -> Option<u64> {
    std::env::var(MAX_TICKS_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_accepts_seconds_suffix() {
        assert_eq!(parse_interval("5s").unwrap(), Duration::from_secs(5));
    }

    #[test]
    fn parse_interval_rejects_zero() {
        assert!(parse_interval("0s").is_err());
    }

    #[test]
    fn clear_mode_prefixes_terminal_redraw_sequence() {
        let rendered = render_cycle(&serde_json::json!({"ok": true}), true).unwrap();
        assert!(rendered.starts_with(CLEAR_SCREEN));
        assert!(rendered.ends_with('\n'));
    }

    #[test]
    fn append_mode_emits_plain_json_line() {
        let rendered = render_cycle(&serde_json::json!({"ok": true}), false).unwrap();
        assert!(!rendered.starts_with(CLEAR_SCREEN));
        assert_eq!(rendered.lines().count(), 1);
    }
}
