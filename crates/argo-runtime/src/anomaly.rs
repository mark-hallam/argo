//! Session-level worker anomaly scoring.
//!
//! This is deliberately separate from `argo_store::anomaly`, which
//! tracks completed action samples. The runtime gate watches the
//! outer worker loop as it runs and can pause a session before it
//! burns more steps.

/// Runtime policy for session-level anomaly detection.
#[derive(Debug, Clone, PartialEq)]
pub struct AnomalyPolicy {
    /// Whether a threshold crossing should pause the worker loop.
    pub auto_pause: bool,
    /// Score threshold in `[0.0, 1.0]` that triggers pause.
    pub threshold: f64,
    /// Repeated near-identical step outputs that count as a loop.
    pub repeated_step_limit: u32,
    /// Single-step latency above this cap contributes a spike score.
    pub max_step_latency_ms: u64,
}

impl Default for AnomalyPolicy {
    fn default() -> Self {
        Self {
            auto_pause: false,
            threshold: 0.85,
            repeated_step_limit: 3,
            max_step_latency_ms: 120_000,
        }
    }
}

impl AnomalyPolicy {
    /// Clamp operator-provided values to safe runtime ranges.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        if !self.threshold.is_finite() {
            self.threshold = 0.85;
        }
        self.threshold = self.threshold.clamp(0.0, 1.0);
        self.repeated_step_limit = self.repeated_step_limit.max(2);
        self.max_step_latency_ms = self.max_step_latency_ms.max(1_000);
        self
    }
}

/// One observed worker-loop step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSignal {
    /// One-based step number.
    pub step: u32,
    /// Assistant text emitted by the step.
    pub text: String,
    /// Wall-clock runtime for the step.
    pub latency_ms: u64,
}

/// Score returned after folding one step.
#[derive(Debug, Clone, PartialEq)]
pub struct AnomalyDecision {
    /// Highest anomaly score seen for this step.
    pub score: f64,
    /// Human-readable reasons that contributed to the score.
    pub reasons: Vec<String>,
    /// Whether the worker loop should pause now.
    pub should_pause: bool,
}

impl AnomalyDecision {
    fn normal() -> Self {
        Self {
            score: 0.0,
            reasons: Vec::new(),
            should_pause: false,
        }
    }
}

/// Stateful scorer for one worker session.
#[derive(Debug, Default)]
pub struct SessionAnomalyState {
    last_fingerprint: Option<String>,
    repeated_steps: u32,
    consecutive_tool_budget_hits: u32,
    consecutive_empty_steps: u32,
}

impl SessionAnomalyState {
    /// Fold a step into the state and return the current decision.
    #[must_use]
    pub fn observe(&mut self, signal: &StepSignal, policy: &AnomalyPolicy) -> AnomalyDecision {
        let policy = policy.clone().sanitized();
        let mut decision = AnomalyDecision::normal();
        self.update_repetition(signal);
        self.update_counters(signal);

        if self.repeated_steps >= policy.repeated_step_limit {
            decision.score = decision.score.max(0.9);
            decision.reasons.push(format!(
                "{} repeated near-identical step outputs",
                self.repeated_steps
            ));
        }
        if self.consecutive_tool_budget_hits >= 2 {
            decision.score = decision.score.max(0.82);
            decision.reasons.push(format!(
                "{} consecutive tool-budget exhaustions",
                self.consecutive_tool_budget_hits
            ));
        }
        if self.consecutive_empty_steps >= 3 {
            decision.score = decision.score.max(0.74);
            decision.reasons.push(format!(
                "{} consecutive empty assistant outputs",
                self.consecutive_empty_steps
            ));
        }
        if signal.latency_ms >= policy.max_step_latency_ms {
            decision.score = decision.score.max(0.8);
            decision.reasons.push(format!(
                "step {} latency {}ms exceeded {}ms",
                signal.step, signal.latency_ms, policy.max_step_latency_ms
            ));
        }

        decision.should_pause = policy.auto_pause && decision.score >= policy.threshold;
        decision
    }

    fn update_repetition(&mut self, signal: &StepSignal) {
        let fp = fingerprint(&signal.text);
        if fp.is_empty() {
            self.repeated_steps = 0;
            self.last_fingerprint = None;
            return;
        }
        if self.last_fingerprint.as_deref() == Some(fp.as_str()) {
            self.repeated_steps += 1;
        } else {
            self.repeated_steps = 1;
            self.last_fingerprint = Some(fp);
        }
    }

    fn update_counters(&mut self, signal: &StepSignal) {
        if signal.text.contains("tool-round budget exhausted") {
            self.consecutive_tool_budget_hits += 1;
        } else {
            self.consecutive_tool_budget_hits = 0;
        }
        if signal.text.trim().is_empty() {
            self.consecutive_empty_steps += 1;
        } else {
            self.consecutive_empty_steps = 0;
        }
    }
}

fn fingerprint(text: &str) -> String {
    text.split_whitespace()
        .take(80)
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
