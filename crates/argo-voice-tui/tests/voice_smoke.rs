//! End-to-end smoke for the voice runner.
//!
//! Two halves:
//!
//! 1. Always-on Mock-driven smoke (runs in `cargo test`): builds a
//!    realistic pending-approval list, drives the runner with a
//!    canned voice script (`Argo approve Maria` / `yes` / `Argo
//!    quit`), asserts the resulting `TickOutcome` sequence.
//!
//! 2. Whisper-native integration smoke: gated behind both the
//!    `whisper-native` feature AND the env var `ARGO_VOICE_INTEGRATION=1`.
//!    Fires only when both are set, validating that `WhisperRecognizer::new`
//!    can actually load a real ggml model file from
//!    `ARGO_WHISPER_MODEL`. Hands a 0.5-second pre-supplied PCM
//!    buffer (silence) and just confirms the loop completes
//!    without error — full-fidelity transcription verification is
//!    a manual operator check (no model is bundled in the repo).

use argo_voice_tui::{
    run_one_tick, MockRecognizer, PendingApproval, RunnerConfig, TickOutcome,
};

fn fixture() -> Vec<PendingApproval> {
    vec![
        PendingApproval {
            token: "tok-maya".into(),
            worker_name: "Maya".into(),
            action_summary: "send invoice reminder".into(),
            spend_usd: None,
        },
        PendingApproval {
            token: "tok-maria".into(),
            worker_name: "Maria Hernandez".into(),
            action_summary: "log payment receipt".into(),
            spend_usd: Some(25.0),
        },
    ]
}

#[test]
fn smoke_approve_then_quit() {
    // Operator approves Maria, then quits.
    let mut r = MockRecognizer::new([
        "Argo approve Maria",
        "yes",
        "Argo quit",
    ]);
    let cfg = RunnerConfig::default();
    let pending = fixture();

    let t1 = run_one_tick(&mut r, &pending, 0, &cfg);
    assert_eq!(t1, TickOutcome::Approved("tok-maria".into()));

    let t2 = run_one_tick(&mut r, &pending, 0, &cfg);
    assert_eq!(t2, TickOutcome::Quit);
}

#[test]
fn smoke_reprompt_then_approve_current() {
    // Operator says nonsense, then says "approve" without naming
    // a worker — should target the focused entry (Maya at index 0).
    let mut r = MockRecognizer::new([
        "Argo schedule a meeting",
        "Argo approve",
        "yes",
    ]);
    let cfg = RunnerConfig::default();
    let pending = fixture();

    let t1 = run_one_tick(&mut r, &pending, 0, &cfg);
    let TickOutcome::Reprompted(_) = t1 else {
        panic!("expected reprompt for unknown phrase, got {:?}", t1);
    };

    let t2 = run_one_tick(&mut r, &pending, 0, &cfg);
    assert_eq!(t2, TickOutcome::Approved("tok-maya".into()));
}

#[cfg(feature = "whisper-native")]
#[test]
fn whisper_recognizer_loads_model_when_env_set() {
    use argo_voice_tui::recognizer::Recognizer;
    use argo_voice_tui::recognizer_whisper::WhisperRecognizer;

    if std::env::var("ARGO_VOICE_INTEGRATION").ok().as_deref() != Some("1") {
        eprintln!("skipping: ARGO_VOICE_INTEGRATION!=1");
        return;
    }
    let Ok(model) = std::env::var("ARGO_WHISPER_MODEL") else {
        eprintln!("skipping: ARGO_WHISPER_MODEL not set");
        return;
    };
    if !std::path::Path::new(&model).exists() {
        eprintln!("skipping: ARGO_WHISPER_MODEL points at non-existent path: {model}");
        return;
    }
    // 0.5 sec of silence at 16 kHz; whisper transcribes it to "" or
    // a short hallucination. We just want WhisperContext::new +
    // create_state + full to round-trip without erroring.
    let silence = vec![0.0_f32; 8000];
    let mut emitted = false;
    let next = Box::new(move || -> Result<Vec<f32>, String> {
        if emitted {
            return Err("test fixture exhausted".into());
        }
        emitted = true;
        Ok(silence.clone())
    });
    let mut rec = WhisperRecognizer::new(&model, next, Some("en".into()))
        .expect("WhisperRecognizer::new");
    let _ = rec.transcribe_next().expect("transcribe_next");
}
