# ws4-gateway-stripe-env-plumb-from-envlocal

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ws4-gateway-stripe-env-plumb-from-envlocal`.

## Why (WS4 billing — VERIFIED gap 2026-06-09)
The live gateway launch path passes **no Stripe secrets** into the running
process, so even once the operator has real Stripe keys, the billing endpoints
fail closed in production. `argo-billing` reads its secrets from process env at
construction (`crates/argo-billing/src/stripe_client.rs`):
- `STRIPE_SECRET_KEY_ENV = "STRIPE_SECRET_KEY"` (line 17) — `read_required`, errors `MissingEnv` if unset/empty (lines 39, 53-56).
- `STRIPE_WEBHOOK_SECRET_ENV = "STRIPE_WEBHOOK_SECRET"` (line 19) — same.
- `STRIPE_BASE_URL_ENV = "STRIPE_BASE_URL"` (line 21) — optional, has a default.

So `POST /api/v1/billing/{checkout,portal,webhook}`
(`crates/argo-cli/src/handlers/gateway/billing.rs`) cannot construct a Stripe
client and will 500 in the live process whenever these vars are absent. The SPA
also reads `NEXT_PUBLIC_STRIPE_PRICE_PRO` / `NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE`
(`web/app/(app)/billing/subscribe/page.tsx:19-22`) — those are build-time SPA
env, but the gateway launch should still surface them from `env.local` for
parity if present.

VERIFIED (grep against `eddcab780`):
- `rg -n 'STRIPE_' scripts/restart-live-gateway.sh` → **ZERO hits**. The script only `source`s `$ARGO_HOME/env.local` wholesale (lines 182, 188) and bootstraps `SKYCOT_DB_KEY_BASE64` + an LLM `*_API_KEY` (lines 154-178); it never names/exports the Stripe vars or warns when they are absent.
- `rg -n 'STRIPE_' ~/Library/LaunchAgents/com.skycot.gateway.plist` → **ZERO hits** (or plist absent).

Net effect: nobody can pay on the live box even after the operator adds Stripe
keys, with no honest signal that billing is unconfigured.

## Scope (agent-doable HALF only)
The **values** are operator-owed secrets — this brief does NOT create, hardcode,
or invent any key. It only teaches the launch path to **surface from
`env.local` if present** and **honestly skip if absent**. The operator pastes
their real keys into `~/.argo/env.local` separately.

## Grep-before-write
- `rg -n 'STRIPE_|env.local|ENV_LOCAL_FILE|source ' scripts/restart-live-gateway.sh` — confirm the existing `source "$ENV_LOCAL_FILE"` mechanism (lines 182, 188) is what we extend; do NOT add a second sourcing path.
- `rg -n 'pub const STRIPE_.*_ENV' crates/argo-billing/src/stripe_client.rs` — the canonical var names the gateway reads (do not rename; mirror exactly).
- `rg 'pub (struct|enum|fn) ' scripts/` is N/A (bash). Bash equivalent — confirm no parallel helper already exists: `rg -n 'plumb_stripe|export_stripe|STRIPE' scripts/` → must stay limited to this script (no duplicate plumbing elsewhere).
- Pattern to mirror: the way the script already `source`s `env.local` and the way the gateway inherits the script's exported env (so any var exported before the gateway spawn is visible to it).

## Fix (additive, name consumers)
1. In `scripts/restart-live-gateway.sh`, after `env.local` is sourced (post line 188), add a small block that, **for each of** `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and the SPA-parity vars `NEXT_PUBLIC_STRIPE_PRICE_PRO` / `NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE` (and optional `STRIPE_BASE_URL`): if the var is set+non-empty in the environment (it will be if present in `env.local`, since that file is sourced), `export` it so the spawned gateway inherits it; if unset/empty, **emit one honest log line** (e.g. `log "      STRIPE_SECRET_KEY not set in env.local — billing endpoints will fail-closed until set"`) and continue. No error, no exit.
2. Do NOT hardcode any key value. Do NOT write keys into `env.local` during bootstrap — only **comment placeholders** may be appended to the bootstrap template block (lines ~154-178), mirroring the existing `# export ANTHROPIC_API_KEY='sk-ant-...'` comment lines (so the operator sees where to paste). Adding commented placeholders is optional but encouraged for discoverability.
3. Keep names EXACT — `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET` must match `stripe_client.rs:17,19` byte-for-byte or the plumb is a no-op.
4. Consumer: the gateway process spawned later in the same script; it reads these via `argo-billing::stripe_client` at billing-endpoint construction. No Rust change, no plist change required (the script is the launch path; if the plist is the active launcher, note that as a follow-up, do not edit it here).

Keep the diff to the one script (a ~10-20 line block + optional comment placeholders). Additive only; no behavior change when the vars are unset beyond the new honest log line.

## Acceptance (truth-bounded; checkable verifier; < 500 LOC)
1. **VERIFIER — present case**: in a throwaway sandbox, write a temp `env.local` exporting `STRIPE_SECRET_KEY=sk_test_dummy` + `STRIPE_WEBHOOK_SECRET=whsec_dummy` (+ the two `NEXT_PUBLIC_STRIPE_PRICE_*` dummies), point `ARGO_HOME` at the temp dir, and run the script in a **dry-run / source-and-echo mode** that stops before actually spawning the gateway (e.g. add a guard env like `RESTART_GATEWAY_DRY_RUN=1` that, after the plumb block, prints the exported Stripe-relevant env and exits 0 — or extract the plumb block into a `plumb_stripe_env()` function and call it from a tiny test harness). Assert the printed/exported env **contains** `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` with the dummy values. This is a checkable verifier (grep the echoed env), not a model self-report.
2. **VERIFIER — absent case**: same harness with NO Stripe vars in `env.local` → the script exits 0, prints the honest "not set" log line(s), and exports nothing Stripe-related (grep the echoed env confirms absence). No error, no non-zero exit.
3. No real key ever appears in the repo: `rg -n 'sk_live|sk_test_[A-Za-z0-9]{10,}|whsec_[A-Za-z0-9]{10,}' scripts/ docs/build-plan/streams/ws4-gateway-stripe-env-plumb-from-envlocal.md` returns only the dummy literals used in the verifier harness (or nothing committed).
4. `bash -n scripts/restart-live-gateway.sh` parses clean (shell syntax check); existing script behavior on a normal run is unchanged (DB-key + LLM-key bootstrap still works).

## On ship (same commit)
- Update the `FEATURE_MATRIX.md` billing/live-gateway row note to record Stripe-env plumbing from `env.local` (status badge + commit SHA + `scripts/restart-live-gateway.sh`). Do NOT regenerate/commit `docs/skycot/feature-matrix.html` (DEC-043 — it is gitignored).

## Effort
S (one launch script, ~10-20 line additive block + a small dry-run verifier hook; no Rust, no plist edit).
