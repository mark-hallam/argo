//! Genesis hire-gate LLM safety-signal client.
//!
//! Split out of `routes_genesis_gate` for LOC headroom — **pure extraction,
//! no behaviour change.** Holds the [`SafetySignal`] dialer and its
//! [`GenesisJudgeClient`] implementation over the shared provider transport
//! (the same machinery the archetype classifier + Pitch Agent use). The gate
//! (`routes_genesis_gate`) calls [`run_safety_signal`]; the transport-failure
//! flag lets the gate tell a real `block` verdict apart from a degraded
//! could-not-run (→ `needs_review`, fail SAFE).

use super::routes_genesis_stream_llm::{complete_once, resolve_provider_id};
use argo_genesis::{
    judge_genesis_draft, GenesisJudgeClient, GenesisJudgeVerdict, JudgeInput, SafetySignal,
};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

/// Dial the LLM safety judge and normalise its outcome into a
/// [`SafetySignal`]. A transport/provider failure (the common CI / no-key
/// case) becomes [`SafetySignal::Unavailable`] (degraded → needs_review),
/// distinct from a real `block`.
pub(super) async fn run_safety_signal(input: JudgeInput<'_>) -> SafetySignal {
    let provider_id = resolve_provider_id();
    let client = GateJudgeClient::new(provider_id);
    // `judge_genesis_draft` fail-closes to `Block` on transport error; we
    // want to tell "judge said no" apart from "judge couldn't run", so we
    // inspect the client's own transport-failure flag afterward.
    let verdict = match judge_genesis_draft(&input, &client).await {
        Ok(v) => v,
        Err(e) => {
            return SafetySignal::Unavailable {
                reason: format!("safety judge invocation error: {e}"),
            }
        }
    };
    if client.transport_failed() {
        // The judge fell through to its conservative `Block` because the
        // provider call errored — that is a *could-not-run*, so degrade to
        // needs_review (audited, held in dry-run) instead of a hard block.
        let reason = match verdict {
            GenesisJudgeVerdict::Block { reason } => reason,
            _ => "provider transport failed".to_string(),
        };
        return SafetySignal::Unavailable { reason };
    }
    match verdict {
        GenesisJudgeVerdict::Allow => SafetySignal::Allow,
        GenesisJudgeVerdict::AllowWithDisclaimer { reason } => {
            SafetySignal::AllowWithDisclaimer { reason }
        }
        GenesisJudgeVerdict::Block { reason } => SafetySignal::Block { reason },
    }
}

/// [`GenesisJudgeClient`] over the shared provider transport (same machinery
/// as the archetype classifier). Records whether the underlying provider
/// round-trip errored so the caller can distinguish a real `block` verdict
/// from a degraded could-not-run.
struct GateJudgeClient {
    provider_id: String,
    transport_failed: AtomicBool,
}

impl GateJudgeClient {
    fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            transport_failed: AtomicBool::new(false),
        }
    }

    fn transport_failed(&self) -> bool {
        self.transport_failed.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl GenesisJudgeClient for GateJudgeClient {
    async fn judge(&self, prompt: &str) -> ArgoResult<String> {
        match complete_once(&self.provider_id, prompt).await {
            Ok(text) => Ok(text),
            Err(e) => {
                self.transport_failed.store(true, Ordering::Relaxed);
                Err(e)
            }
        }
    }
}
