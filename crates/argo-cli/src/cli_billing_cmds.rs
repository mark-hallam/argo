//! `argo billing replay` + `argo billing dlq` — operator surfaces
//! for recovering from Stripe webhook failures (Stream
//! `codex-billing-webhook-resilience`). Replay reads
//! `billing_event_dead_letter` and re-applies events without
//! re-calling Stripe.

use clap::Subcommand;

/// `argo billing <subcommand>` — Stripe billing operator surfaces.
#[derive(Subcommand, Debug)]
pub enum BillingCmd {
    /// Replay one or more dead-lettered Stripe events. Reads from
    /// `billing_event_dead_letter`; no Stripe API call.
    Replay(BillingReplayArgs),
    /// Inspect or resolve dead-lettered webhook events.
    Dlq {
        #[command(subcommand)]
        cmd: BillingDlqCmd,
    },
}

/// Args for `argo billing replay`. Exactly one of `--event-id` or
/// `--since` must be set.
#[derive(clap::Args, Debug)]
pub struct BillingReplayArgs {
    /// Replay a single event by its Stripe `evt_…` id.
    #[arg(long, conflicts_with = "since")]
    pub event_id: Option<String>,
    /// Replay every unresolved DLQ entry whose `failed_at` is within
    /// this duration of now. Accepts `30m`, `24h`, `7d`.
    #[arg(long, conflicts_with = "event_id")]
    pub since: Option<String>,
    /// Per-batch row cap (replay-since only). Default 100.
    #[arg(long, default_value_t = 100)]
    pub limit: i64,
    /// Emit a JSON envelope to stdout instead of human text.
    #[arg(long)]
    pub json: bool,
}

/// `argo billing dlq <subcommand>` — dead-letter queue ops.
#[derive(Subcommand, Debug)]
pub enum BillingDlqCmd {
    /// List dead-letter entries with reasons + retry counts.
    List {
        /// Only show entries failed within this duration (e.g.
        /// `24h`). Default: no lower bound.
        #[arg(long)]
        since: Option<String>,
        /// Include rows that have already been resolved.
        #[arg(long)]
        include_resolved: bool,
        /// Row cap. Default 100.
        #[arg(long, default_value_t = 100)]
        limit: i64,
        /// Emit JSON instead of human text.
        #[arg(long)]
        json: bool,
    },
    /// Mark a DLQ entry resolved without running replay. Use when
    /// the operator has fixed the underlying state out-of-band and
    /// wants the entry off the queue.
    Resolve {
        /// Stripe `evt_…` id.
        #[arg(long)]
        event_id: String,
        /// Free-text note recorded with the resolution.
        #[arg(long)]
        note: Option<String>,
        /// Emit JSON instead of human text.
        #[arg(long)]
        json: bool,
    },
}
