//! `argo demo` subcommand tree — currently a single `seed` variant
//! that invokes the canonical v1 demo workspace seed body in
//! [`argo_demo_seed::run_canonical_v1_seed`].
//!
//! Lives in its own file so the parent `cli.rs` and
//! `cli_command_enum.rs` stay narrowly scoped per the brief's
//! may-touch list and the 500-LOC per-file invariant.

use clap::Subcommand;

/// `argo demo <cmd>` — operator-facing demo workspace primitives.
#[derive(Subcommand, Debug)]
pub enum DemoCmd {
    /// Seed a real workspace with the canonical v1.0 demo data
    /// (5 named workers, the Scene-5 combo workflow, the Scene-4
    /// competitor pricing skill catalog, Pass A real-LLM scaffolding
    /// rows, and Pass B dressing audit rows). See the brief
    /// `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md`
    /// and the function-vs-dressing reframe at
    /// `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.
    #[command(after_help = "Example:\n  \
        # Seed Pass B + scaffolding only (no provider key required):\n  \
        argo demo seed --tenant 01900000-0000-7000-8000-000000000000 \\\n      \
        --skip-pass-a-turns\n  \
        # Full seed with real-LLM Pass A turns (requires sandbox key):\n  \
        ARGO_DEMO_SEED_PROVIDER_KEY=sk-sandbox-... \\\n      \
        argo demo seed --tenant 01900000-0000-7000-8000-000000000000")]
    Seed {
        /// Tenant id to seed (must already be provisioned).
        #[arg(long)]
        tenant: String,
        /// Seed profile id. Currently only `v1-demo-canonical` is
        /// shipped.
        #[arg(long, default_value = "v1-demo-canonical")]
        profile: String,
        /// Number of Pass A turn-shaped audit rows to insert. Each
        /// "turn" emits 3 rows (TurnStart + AssistantMessage +
        /// TurnEnd). Brief calls for 10-20 turns; default 12.
        #[arg(long, default_value_t = 12)]
        pass_a_turns: usize,
        /// Number of Pass B dressing audit rows to insert. Brief
        /// calls for 100-200 rows; default 150.
        #[arg(long, default_value_t = 150)]
        dressing_rows: usize,
        /// Skip Pass A entirely. Useful for smoke tests and offline
        /// dry runs that should not require a real provider key.
        #[arg(long)]
        skip_pass_a_turns: bool,
        /// Emit the seed report as JSON (one object on stdout).
        #[arg(long)]
        json: bool,
    },
}
