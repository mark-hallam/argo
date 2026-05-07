//! `argo voice` — local voice TUI handler.
//!
//! Wires the [`argo_voice_tui`] crate's runner up to the operator
//! terminal. Two operating modes:
//!
//! 1. **Mock script mode** (`--mock-script <path>`): reads
//!    `{"script": ["argo approve maya", "yes", ...]}` from JSON
//!    and feeds it through [`argo_voice_tui::run_one_tick`] until
//!    the script is exhausted or a `Quit` outcome lands. No mic,
//!    no whisper-rs needed — perfect for contributors testing
//!    the loop without rebuilding with `--features
//!    whisper-native`.
//! 2. **Live mode** (default): with the `whisper-native` feature
//!    enabled, opens the default mic via cpal, transcribes via
//!    whisper-rs, drives the runner. Without the feature, prints
//!    the rebuild hint and falls back to a no-op completion (the
//!    pre-existing TUI list view is the recommended fallback;
//!    operators still launch it via `argo tui`).

use serde::{Deserialize, Serialize};

use argo_voice_tui::{
    run_one_tick, MockRecognizer, PendingApproval, RunnerConfig, TickOutcome,
};

#[derive(Debug, Deserialize)]
struct MockScript {
    script: Vec<String>,
    #[serde(default)]
    pending: Vec<PendingApprovalInput>,
    #[serde(default)]
    focus_index: usize,
}

#[derive(Debug, Deserialize, Clone)]
struct PendingApprovalInput {
    token: String,
    worker_name: String,
    action_summary: String,
    #[serde(default)]
    spend_usd: Option<f64>,
}

impl From<PendingApprovalInput> for PendingApproval {
    fn from(p: PendingApprovalInput) -> Self {
        Self {
            token: p.token,
            worker_name: p.worker_name,
            action_summary: p.action_summary,
            spend_usd: p.spend_usd,
        }
    }
}

/// JSON envelope for `--json` mode. One object per outcome line
/// (NDJSON when multiple ticks fire).
#[derive(Debug, Serialize)]
struct VoiceOutcomeJson {
    kind: &'static str,
    payload: Option<String>,
}

impl From<&TickOutcome> for VoiceOutcomeJson {
    fn from(o: &TickOutcome) -> Self {
        match o {
            TickOutcome::Approved(t) => Self {
                kind: "approved",
                payload: Some(t.clone()),
            },
            TickOutcome::Denied(t) => Self {
                kind: "denied",
                payload: Some(t.clone()),
            },
            TickOutcome::DetailsRequested(t) => Self {
                kind: "details_requested",
                payload: Some(t.clone()),
            },
            TickOutcome::Next => Self {
                kind: "next",
                payload: None,
            },
            TickOutcome::Quit => Self {
                kind: "quit",
                payload: None,
            },
            TickOutcome::Reprompted(m) => Self {
                kind: "reprompted",
                payload: Some(m.clone()),
            },
            TickOutcome::Error(m) => Self {
                kind: "error",
                payload: Some(m.clone()),
            },
        }
    }
}

/// Run `argo voice`. Returns process exit code (0 = clean Quit,
/// 1 = error / fallback fired).
pub fn run(mock_script: Option<String>, json: bool) -> i32 {
    if let Some(path) = mock_script {
        return run_mock(&path, json);
    }
    run_live(json)
}

fn run_mock(path: &str, json: bool) -> i32 {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("argo voice: read --mock-script {path}: {e}");
            return 1;
        }
    };
    let script: MockScript = match serde_json::from_str(&raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("argo voice: parse --mock-script {path}: {e}");
            return 1;
        }
    };
    let pending: Vec<PendingApproval> = script.pending.into_iter().map(Into::into).collect();
    let mut recognizer = MockRecognizer::new(script.script);
    let cfg = RunnerConfig::default();
    let mut focus = script.focus_index;
    loop {
        let outcome = run_one_tick(&mut recognizer, &pending, focus, &cfg);
        emit(&outcome, json);
        match outcome {
            TickOutcome::Quit => return 0,
            TickOutcome::Error(_) => return 1,
            TickOutcome::Next if focus + 1 < pending.len() => focus += 1,
            _ => {}
        }
        if recognizer.remaining() == 0 {
            return 0;
        }
    }
}

fn run_live(json: bool) -> i32 {
    #[cfg(feature = "whisper-native")]
    {
        run_live_whisper(json)
    }
    #[cfg(not(feature = "whisper-native"))]
    {
        let msg = "argo voice: live mic capture requires `--features whisper-native`. \
                   Rebuild with `cargo build -p argo-cli --features \
                   argo-voice-tui/whisper-native`. Falling back to TUI list view — \
                   run `argo tui` for the existing terminal approval UI.";
        if json {
            let line = serde_json::json!({
                "kind": "fallback",
                "payload": msg,
            });
            println!("{line}");
        } else {
            eprintln!("{msg}");
        }
        let _ = json;
        1
    }
}

#[cfg(feature = "whisper-native")]
fn run_live_whisper(json: bool) -> i32 {
    use argo_voice_tui::audio::{enumerate_input_devices, record_one_phrase};
    use argo_voice_tui::recognizer_whisper::WhisperRecognizer;

    let devices = enumerate_input_devices();
    if devices.is_empty() {
        let msg =
            "argo voice: no input device found (mic permission denied or no hardware). \
             Falling back to TUI list view — run `argo tui`.";
        if json {
            let line = serde_json::json!({"kind": "fallback", "payload": msg});
            println!("{line}");
        } else {
            eprintln!("{msg}");
        }
        return 1;
    }
    let model = match std::env::var("ARGO_WHISPER_MODEL") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => {
            eprintln!(
                "argo voice: ARGO_WHISPER_MODEL is not set. Point it at a ggml whisper \
                 model file (e.g. ggml-base.en.bin) — download from \
                 https://huggingface.co/ggerganov/whisper.cpp"
            );
            return 1;
        }
    };
    let next = Box::new(|| -> Result<Vec<f32>, String> { record_one_phrase(3.0) });
    let mut recognizer = match WhisperRecognizer::new(&model, next, Some("en".into())) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("argo voice: WhisperRecognizer::new: {e}");
            return 1;
        }
    };
    let pending: Vec<PendingApproval> = Vec::new();
    let cfg = RunnerConfig::default();
    eprintln!(
        "argo voice: listening (input={}). Say `Argo approve <name>` / `Argo \
         deny <name>` / `Argo next` / `Argo quit`.",
        devices.first().map(String::as_str).unwrap_or("?")
    );
    loop {
        let outcome = run_one_tick(&mut recognizer, &pending, 0, &cfg);
        emit(&outcome, json);
        if matches!(outcome, TickOutcome::Quit | TickOutcome::Error(_)) {
            return if matches!(outcome, TickOutcome::Quit) { 0 } else { 1 };
        }
    }
}

fn emit(outcome: &TickOutcome, json: bool) {
    if json {
        let line: VoiceOutcomeJson = outcome.into();
        match serde_json::to_string(&line) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("argo voice: render outcome: {e}"),
        }
        return;
    }
    match outcome {
        TickOutcome::Approved(t) => println!("approved {t}"),
        TickOutcome::Denied(t) => println!("denied {t}"),
        TickOutcome::DetailsRequested(t) => println!("details {t}"),
        TickOutcome::Next => println!("next"),
        TickOutcome::Quit => println!("quit"),
        TickOutcome::Reprompted(m) => eprintln!("reprompt: {m}"),
        TickOutcome::Error(m) => eprintln!("error: {m}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn mock_script_runs_to_quit() {
        let mut tf = tempfile::Builder::new()
            .suffix(".json")
            .tempfile()
            .expect("tempfile");
        let body = serde_json::json!({
            "script": ["Argo approve Maya", "yes", "Argo quit"],
            "pending": [
                {
                    "token": "tok-maya",
                    "worker_name": "Maya",
                    "action_summary": "send invoice reminder",
                }
            ],
            "focus_index": 0,
        });
        tf.write_all(body.to_string().as_bytes()).expect("write");
        let path = tf.path().to_string_lossy().to_string();
        let code = run(Some(path), true);
        assert_eq!(code, 0);
    }

    #[test]
    fn mock_script_invalid_returns_one() {
        let code = run(Some("/nonexistent/path/to/script.json".into()), true);
        assert_eq!(code, 1);
    }

    #[test]
    fn outcome_json_round_trip() {
        let o = TickOutcome::Approved("tok-1".into());
        let j: VoiceOutcomeJson = (&o).into();
        assert_eq!(j.kind, "approved");
        assert_eq!(j.payload, Some("tok-1".into()));
    }
}
