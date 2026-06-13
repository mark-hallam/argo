# Disabled workflows policy

**Decision date:** 2026-05-17 (operator-locked); extended 2026-05-21 to cover ALL workflows (including formerly-active `ci.yml` + 4 siblings).
**Status:** **ALL workflows in this directory are now disabled** — both on disk (every `.yml` is `.yml.disabled`) AND at the GitHub repo level (Settings → Actions → "Disable actions"). Zero workflows can run.
**Audit reference:** Closes 2026-05-17 read-only audit finding #12 (`docs/skycot/audit/2026-05-17-readonly-codebase-audit.md`). 2026-05-21 extension: operator silencing GHA failure-email storm during v1.0 build; every workflow was failing on every push (no billing = no runner minutes = guaranteed failure = 1 email per run).

## Rationale

GitHub Actions billing is operator-side blocked. Admin-merge is the v1.0 strategy. Every workflow in this directory is disabled with intent — not waiting for a "re-enable before GA" flip — until either (a) GHA billing is restored, (b) the operator stands up a self-hosted runner, or (c) the underlying workflow is no longer load-bearing.

**2026-05-21 supplemental:** the 5 formerly-active workflows (`ci.yml`, `deploy-smoke.yml`, `cli-reference-drift.yml`, `audit.yml`, `deny.yml`) were generating 40-60 failure emails per hour during the active merge wave. Disabled at the operator's request to restore inbox sanity. The signal these provided was zero (every run failed for billing reasons, not real bugs). Local verification gates (`scripts/conductor-pre-merge-gate.sh` + worker-side pre-push `cargo clippy`/`cargo test`) cover the verification surface during v1 dev.

## Per-workflow classification

| File | Classification | Why |
|---|---|---|
| `bench.yml.disabled` | **keep-disabled** (v1) | Bench infrastructure is local-developer-only for v1. Criterion benches from #1365 run on-demand via `cargo bench`. CI bench cadence is post-launch. |
| `deploy-prod.yml.disabled` | **keep-disabled** (v1) | Production deployment is manual via `flyctl deploy` for v1. Automated deploy gate is post-launch. |
| `deploy-spa.yml.disabled` | **keep-disabled** (v1) | SPA deployment is manual via Vercel CLI / dashboard for v1. Automated SPA-deploy gate is post-launch. |
| `docs.yml.disabled` | **keep-disabled** (v1) | mdBook + Cargo doc workflows; SKYCOT docs are markdown-direct for v1; no rustdoc publishing surface needed pre-launch. |
| `fuzz-nightly.yml.disabled` | **keep-disabled** (v1) | Nightly cargo-fuzz across argo-types/argo-config/argo-audit. Operator runs locally; nightly CI cost not justified pre-launch. |
| `oci-smoke.yml.disabled` | **keep-disabled** (v1) | No OCI release target for v1. Re-evaluate when container distribution is on the roadmap. |
| `real-llm-smoke.yml.disabled` | **keep-disabled** (v1) | Real-provider LLM smoke; cost-gated (real Anthropic/OpenAI calls). Operator-triggered only. |
| `release.yml.disabled` | **keep-disabled** (v1) | Release tagging + asset upload. Manual release for v1.0 ship; automate post-launch. |
| `schema-doc-check.yml.disabled` | **keep-disabled** (v1) | Pending JSON schema export surface; not on v1 critical path. |
| `silent-skip-nightly.yml.disabled` | **keep-disabled** (v1) | Tracks `#[ignore]` accumulation; cosmetic-quality not load-bearing. |
| `soak.yml.disabled` | **keep-disabled** (v1) | Multi-hour soak tests; operator-triggered. v1 load-soak coverage comes via `codex-gateway-load-soak-baseline` (manual workflow_dispatch). |
| `load-soak.yml.disabled` | **keep-disabled** (v1) | Operator-triggered via `workflow_dispatch`; expensive; not needed for routine merge gating. Added by PR #1811 (`codex-gateway-load-soak-baseline`). |
| `ci.yml.disabled` | **keep-disabled** (v1)  — *newly disabled 2026-05-21* | fmt + clippy + test matrix + coverage + loc-budget + prepared-statements + parallel-pub-types-enforce + audit-sink-tenant-guard. Replaced by local `scripts/conductor-pre-merge-gate.sh` + worker-side pre-push gates during v1.0 build. |
| `deploy-smoke.yml.disabled` | **keep-disabled** (v1)  — *newly disabled 2026-05-21* | Deployment smoke; nothing to deploy yet for v1. Will re-enable when K5 deploy-smoke-test ships. |
| `cli-reference-drift.yml.disabled` | **keep-disabled** (v1)  — *newly disabled 2026-05-21* | CLI reference docs drift detection; cosmetic for v1 build. Re-enable post-launch. |
| `audit.yml.disabled` | **keep-disabled** (v1)  — *newly disabled 2026-05-21* | Audit-chain CI checks; replaced by local audit-sink-tenant-guard in `pre-merge-gate.sh` + integration tests in `argo-audit`. |
| `deny.yml.disabled` | **keep-disabled** (v1)  — *newly disabled 2026-05-21* | `cargo-deny` license/vulnerability checks; valuable but operator-triggered locally during v1 build. |
| `web-cold-install.yml.disabled` | **keep-disabled** (v1)  — *added 2026-05-22, landed disabled* | Cold pnpm install + typecheck + lint gate that catches `package.json`/`pnpm-lock.yaml` drift the warm-cache `deploy-spa` gate misses (closes web-pnpm-install-missing-on-cold-start). Lands disabled because GHA billing is still operator-blocked; the file is on-disk so it ships ready to flip when billing returns. |
| `windows-build.yml.disabled` | **keep-disabled** (v1)  — *added 2026-06-14 (win-ci-windows-build-job-authored); expanded 2026-06-14 (win-ci-windows-test-job-expand)* | Full Windows surface on `windows-latest`: `cargo build --workspace` (link guard) + an EXPLICIT `cargo build`/`cargo test -p argo-executor-computer-use-windows` step (the crown-jewel native CU-executor crate, incl. the non-input-gated PNG-encode decode test) + `cargo test --workspace --no-fail-fast`. Future-guard for the Windows portability surface the Windows worker fleet (`docs/build-plan/BUILD_STATUS_WINDOWS.md`) is hardening, so a Windows-only regression is caught automatically once CI billing returns. Real SendInput / live-desktop integration stays gated behind `ARGO_NATIVE_WINDOWS_INTEGRATION=1` (not set by this job). Lands disabled (GHA billing operator-blocked); ships on-disk ready to flip. Until then, Windows verification is the per-brief real-Windows `cargo` runs recorded in each fleet PR. |

## Re-enable trigger conditions

Per-workflow flip from `keep-disabled` to active requires ONE of:

1. GHA billing restored OR self-hosted runner stood up (covers cost concern).
2. The workflow's underlying surface ships (e.g. release.yml needs a release process; deploy-prod.yml needs a deploy gate).
3. Post-v1.0 launch (re-evaluate each as part of v1.x scoping).

## Currently active workflows: NONE

**ZERO workflows run today.** Every `*.yml` in this directory is suffixed
`.yml.disabled` (verified) AND GitHub Actions is disabled at the repo level
(Settings → Actions → "Disable actions"). Verification during v1.0 build is
covered entirely by admin-merge + local gates (`scripts/conductor-pre-merge-gate.sh`
+ worker-side pre-push `cargo clippy` / `cargo test`). No CI runs on push.

## Re-enable priority order (when GitHub Actions billing returns)

The 5 workflows that were active until 2026-05-21 are the ones to RE-ENABLE
first, in this order (per the 2026-06-10 test-health audit, F9 — `ci.yml` first):

1. `ci.yml` — fmt, clippy, test matrix, coverage, loc-budget, prepared-statements, error-variant-usage, config-callsite-audit, parallel-pub-types-enforce, audit-sink-tenant-guard. **Re-enable first** — it is the core merge gate.
2. `audit.yml` — cargo audit (RUSTSEC advisories).
3. `deny.yml` — cargo deny (advisories/bans/sources/licenses).
4. `deploy-smoke.yml` — `scripts/smoke-skycot.sh` + optional `--target-url` post-deploy.
5. `cli-reference-drift.yml` — CLI `--help` vs `docs/skycot/cli-reference.md` drift check.

To re-enable a workflow: rename `<name>.yml.disabled` → `<name>.yml` AND
re-enable Actions at the repo level. The remaining `keep-disabled (v1)`
workflows in the table above stay disabled until their underlying surface
ships (see "Re-enable trigger conditions").
