# skill-learn-real-provider-distillation-caller

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh skill-learn-real-provider-distillation-caller`.

## Why (VERIFIED gap 2026-06-09)
The CU→skill learning loop ("teammates build new skills from real work" —
mission **claim 4** + **DEC-042 Stage-2 P-1**) ships a **deterministic
placeholder**, not a real model distillation:
- `crates/argo-skill/src/learn.rs:21-24` — "the offline path here ships a
  deterministic placeholder body the operator can refine before approving."
- `crates/argo-skill/src/self_improve_cron.rs:31-33` + `:189-220` — the cron
  driver runs through `DeterministicDistillationCaller` ("**no live model, no
  network**"), and `self_improve_tick` (`:228-236`) hardcodes
  `&DeterministicDistillationCaller`. The distilled "skill body" is just a
  mechanical reformat of the anchor trajectory's steps — no synthesis.

The real provider transport that genesis / pitch / ghostwriter / marketing use
is `argo_agent_runner::build_transport` + `load_manifest` →
`argo_provider::Transport` (verified: `routes_pitch_ask_llm.rs:33` +
`:285-296` `build_pitch_transport`; `routes_genesis_stream_llm.rs:30,279`;
`routes_ghostwriter_llm.rs`). It is **not** threaded into the distillation
caller. The one production `SkillDistillationCaller` that exists
(`EnvSkillDistillationCaller` in `crates/argo-cli/src/handlers/skill_distill_now.rs:176-224`)
calls a SEPARATE `https://api.anthropic.com/v1/dreams` endpoint, NOT the shared
provider transport, and is **not** wired into the cron loop at all.

The promotion gate ALREADY exists and is real: `run_ab_eval`
(`ab_eval.rs:108`) → `decide_promotion` (`self_improve_loop.rs:220`) over
`PromotionPolicy` (floor `min_candidate_rate: 0.5`, `min_delta: 0.05`,
`max_regression: 0.05` — `self_improve_loop.rs:152-159`), returning
`PromotionDecision::{Promote, Hold, Reject}`. This brief makes the
**distillation** real and keeps that eval gate authoritative — so a distilled
skill must beat the floor before promotion (DEC-042 / NO-THEATER: the verifier
is a measured pass-rate, not a model's self-report). A distilled skill is
**draft-only at birth** (DEC-041/037) — promotion is the only path to live.

## Grep-before-write
- `rg -n 'trait SkillDistillationCaller|async fn distill|DistillationResult|DeterministicDistillationCaller' crates/argo-skill/src/dreams_distillation.rs crates/argo-skill/src/self_improve_cron.rs` — the trait (`dreams_distillation.rs:101`, `async fn distill(&self, cohort: &Cohort) -> ArgoResult<DistillationResult>`), the result shape (`:88` body/model/cost_micros), and the deterministic impl to keep as default.
- `rg -n 'build_transport|load_manifest|build_pitch_transport|resolve_provider_id|complete_once|Transport' crates/argo-cli/src/handlers/gateway/routes_pitch_ask_llm.rs crates/argo-cli/src/handlers/gateway/routes_genesis_stream_llm.rs` — the SHARED provider transport (`argo_agent_runner::build_transport` + `load_manifest`). REUSE this; do NOT add a parallel provider path and do NOT extend the `EnvSkillDistillationCaller` `/v1/dreams` HTTP path (it is a different, narrower transport).
- `rg -n 'self_improve_tick|self_improve_tick_with|decide_promotion|run_ab_eval|PromotionPolicy|PromotionDecision' crates/argo-skill/src/self_improve_cron.rs crates/argo-skill/src/self_improve_loop.rs crates/argo-skill/src/ab_eval.rs` — the tick that consumes the caller + the EXISTING promotion gate (do not re-implement either).
- `rg 'pub (struct|enum|fn) ProviderDistillationCaller' crates/` — MUST be empty before declaring it (this is the new surface; confirm no near-duplicate exists).
- `rg 'pub (struct|enum|fn) (SkillDistillationCaller|DeterministicDistillationCaller|EnvSkillDistillationCaller)' crates/` — confirm the three existing callers so the new one is additive, named distinctly, and the deterministic one stays the offline/test default.

## Fix (additive — deterministic path unchanged)
1. Add a new module (e.g. `crates/argo-cli/src/handlers/provider_distillation_caller.rs`,
   in argo-cli so it can reach `argo_agent_runner`) exposing
   `pub struct ProviderDistillationCaller { provider_id, ... }` that
   `impl SkillDistillationCaller`. Its `distill(cohort)`:
   a. Builds the LLM transport via the SHARED
      `argo_agent_runner::{build_transport, load_manifest}` (the SAME machinery
      `build_pitch_transport` uses — name it, do not fork). Provider id is
      config/env-resolved (mirror `ARGO_PITCH_LLM_PROVIDER`, e.g.
      `ARGO_SKILL_DISTILL_PROVIDER`, default `anthropic`, with `claude-code-cli`
      subprocess as an option — reuse the existing manifest-archetype path).
   b. Renders a synthesis prompt from the cohort's trajectories (members'
      `trajectory` steps + `final_text` + `success_metric`) asking for a
      reusable SKILL.md body that generalises the successful runs.
   c. Returns `DistillationResult { body, model, cost_micros }` (real model id +
      real estimated cost), propagating `ArgoError::Provider` on transport
      failure so a no-key / CI run degrades cleanly (NOT a panic).
2. **Wire it behind config**: add a live-caller selector to the cron driver so
   `self_improve_tick_with` is given `&ProviderDistillationCaller` when the
   live flag is ON, and `&DeterministicDistillationCaller` (the existing
   default) otherwise. The flag defaults **OFF** so CI / demo stay
   deterministic. Keep `self_improve_cron.rs:233` deterministic default as-is;
   the live caller is opt-in.
3. **Eval gate stays authoritative**: the tick's existing
   `run_ab_eval` → `decide_promotion` chain decides promote/hold on MEASURED
   pass-rate. A distilled skill is proposed draft-only; only a `Promote`
   decision (clears `min_candidate_rate` + `min_delta`, no `max_regression`
   breach) graduates it via `promote_with_regression_gate`. Do NOT bypass or
   weaken the gate; do NOT auto-graduate.
4. No new `pub` type parallels the trait — `ProviderDistillationCaller` is the
   ONLY new public surface, and it implements the existing
   `SkillDistillationCaller` trait.

## Acceptance (truth-bounded)
1. Test (`crates/argo-cli` or `crates/argo-skill` with a stub transport): a stub
   provider returning a KNOWN body → `ProviderDistillationCaller::distill`
   yields a `DistillationResult` carrying that body + a non-"deterministic"
   model id (proves the live path actually called the transport, not the
   placeholder).
2. Test: run the self-improve tick with the live caller + the existing eval
   gate, where the stubbed A/B numbers put the candidate **BELOW** the floor
   (`min_candidate_rate`/`min_delta`) → assert `PromotionDecision::Hold` (or
   `Reject` on regression) and the skill is NOT promoted (stays draft).
3. Test: the SAME tick with stubbed numbers **ABOVE** the floor → assert
   `PromotionDecision::Promote` and the skill graduates (eval-gated promotion).
4. Test: the deterministic path still works with **no provider configured** —
   `self_improve_tick(&skills)` (default `DeterministicDistillationCaller`)
   runs end to end, no network, reproducible body (proves the offline default
   is intact).
5. `cargo test -p argo-skill -p argo-cli` green; new file(s) each < 500 LOC.
   Clippy clean on touched files (boundary-note pre-existing
   `routes_settings_credentials` await-holding-lock).

## On ship (same commit)
Update the `FEATURE_MATRIX.md` row for the skill-learning / self-improvement
loop (claim 4 + DEC-042 Stage-2 P-1) to note real provider-backed distillation
is wired behind config + eval-gated (status badge + commit SHA + crate
`argo-skill` / `argo-cli`). Do NOT regenerate `feature-matrix.html` (DEC-043,
gitignored). Note this turns "teammates build new skills" from a template stub
into a real, eval-gated distillation.

## Effort
M — one new caller impl reusing the shared `build_transport` machinery + a
config-gated selector in the existing tick + tests. The eval/security boundary
is load-bearing: the live caller is opt-in (default OFF), distilled skills are
draft-only, and promotion is decided ONLY by the existing measured A/B gate —
never the model's self-report.
