//! `argo-demo-seed` — canonical v1.0 demo workspace seeder.
//!
//! This crate inserts the voiceover-aligned demo data for the v1.0
//! mission demo MP4 (`docs/skycot/demos/v1-mission-mvp.mp4`) into a
//! real workspace via the same gateway primitives the SPA already
//! consumes. The seed is structured in two passes per the operator's
//! function-vs-dressing rule (see
//! `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` and
//! `feedback_no_theater_in_demo` memory):
//!
//! * **Pass A (function-side)** — rows the camera drills into must
//!   be real. Five named workers (Mariah / Peta / Priya / Jamal /
//!   Nori) via [`argo_identity::IdentityStore::create_worker`], the
//!   `customer_research_followup` workflow definition, the
//!   `competitor pricing scrape` skill catalog, and ≥1 foreign-tenant
//!   row across `worker` + `audit_record` + `skill_ir` so Scene 6's
//!   cross-tenant invisibility beat queries real SQL and gets a real
//!   empty result. Real-LLM agent turns are gated behind
//!   `ARGO_DEMO_SEED_PROVIDER_KEY`; with no key configured the seed
//!   refuses to advance past the Pass A scaffolding step unless the
//!   operator passes [`SeedOptions::skip_pass_a_turns`].
//!
//! * **Pass B (dressing)** — rows the camera does NOT drill into may
//!   be fixture-tier. ~100-200 historical `audit_record` rows with
//!   realistic timestamps spread across the past 14 days; the
//!   payloads are synthetic but the rows are internally consistent
//!   (real `worker_id`, real `tenant_id`, valid schema). Approval-
//!   request rows the demo may or may not drill into are inserted
//!   here with linked `WorkerAction`-shaped audit rows.
//!
//! The entry point is [`run_canonical_v1_seed`].
//!
//! Brief: `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md`.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
#![warn(missing_docs)]

mod personas;
mod plan;
mod seed;
mod skills;
mod workflow;

pub use personas::{DemoPersona, DEMO_SEED_PERSONAS};
pub use plan::{SeedOptions, SeedReport};
pub use seed::{run_canonical_v1_seed, seed_workers};

/// Environment variable the CLI handler checks for a sandbox provider
/// key. When unset, [`run_canonical_v1_seed`] fails closed unless
/// [`SeedOptions::skip_pass_a_turns`] is true.
pub const PROVIDER_KEY_ENV: &str = "ARGO_DEMO_SEED_PROVIDER_KEY";

/// Default profile id the CLI passes through. Currently the only
/// profile shipped is `"v1-demo-canonical"`; future profiles
/// (`"smoke"`, `"v1.1-demo"`) hang off this enum-of-strings.
pub const DEFAULT_PROFILE: &str = "v1-demo-canonical";

/// Error type returned by [`run_canonical_v1_seed`] and helpers.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    /// Pass A requires a real provider key and none was configured.
    /// The caller passed [`SeedOptions::skip_pass_a_turns`] = `false`
    /// AND [`PROVIDER_KEY_ENV`] was unset.
    #[error(
        "demo seed Pass A requires a real LLM provider key in env var \
         `{PROVIDER_KEY_ENV}`; configure one or re-run with \
         `--skip-pass-a-turns` to seed Pass B (dressing) rows only"
    )]
    MissingProviderKey,
    /// An unknown profile id was passed.
    #[error("demo seed: unknown profile id `{0}` (known: `{DEFAULT_PROFILE}`)")]
    UnknownProfile(String),
    /// An underlying argo-store / argo-identity / argo-audit call failed.
    #[error(transparent)]
    Argo(#[from] argo_types::ArgoError),
}

/// Convenience result alias.
pub type SeedResult<T> = Result<T, SeedError>;
