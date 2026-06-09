# gateway-public-safe-pitch-ask-rate-limited

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh gateway-public-safe-pitch-ask-rate-limited`.

## Why (VERIFIED gap 2026-06-09)
The landing page's headline conversion device — "Talk to Portia"
(`web/components/marketing/PortiaEmbed.tsx`) — shows **verbatim CANNED answers**
with NO live call. There is no public pitch route:
- `rg -c 'ask/public' crates/` = **0** (verified — no public pitch path exists).
- `/api/v1/pitch/ask` is mounted at `routes_dispatch_post.rs:252-256` inside
  `api_v1_dispatch`, which runs **after** `preflight_dispatch`'s
  `require_identity` bearer gate (`routes_dispatch_post.rs:201`). So the only
  way to reach the truth-bounded answerer is with an operator bearer token — a
  public web visitor cannot call it.
- `PortiaEmbed.tsx`'s own header says a public path "would need a
  gated/rate-limited path." It hasn't been built — so the **viral proof of the
  truth-boundary moat is currently fake** (a hand-written transcript, not a
  live grounded answer).

The reusable answerer already exists and is truth-bounded:
`routes_pitch_ask::ask_response` (`routes_pitch_ask.rs:95`) does empty-question
rejection (`bad_request`), canonical-corpus lookup
(`bundle.index.lookup`), a per-channel `TruthBoundaryPolicy`
(`bundle.policy_for`), and a deterministic `MISS_FALLBACK` line on a corpus
miss (`routes_pitch_ask.rs:77`). Do **NOT** fork a parallel pitch engine —
reuse `ask_response`.

## Grep-before-write
- `rg -n 'pitch/ask|ask_response|MISS_FALLBACK|PitchAskRequest|bad_request' crates/argo-cli/src/handlers/gateway/routes_pitch_ask.rs` — the reusable answerer + its existing input/empty guards + the corpus-miss fallback line.
- `rg -n 'preflight_dispatch|require_identity|routes_wizard|api_v1_dispatch|pitch/ask' crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — the dispatch order. `routes_wizard::dispatch_post` (line 198) is the EXISTING pre-bearer (unauthenticated) lane — mount the public route the SAME way, BEFORE `require_identity`.
- `rg -n 'check_request|extract_tenant|client_ip|429|Too Many Requests' crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs crates/argo-cli/src/handlers/gateway/rate_limit.rs` — the EXISTING 429 path (`rate_limit_tenant::check_request` returns `Some(("429 Too Many Requests", body))`); `rate_limit::extract_tenant` / the IP classifier are the per-caller key source. Reuse the existing limiter — do NOT add a parallel one.
- `rg 'pub (struct|enum|fn) ask_response' crates/argo-cli/src/handlers/gateway/` — confirm `ask_response` is the single answerer; the public handler delegates to it.
- `rg 'pub (struct|enum|fn) PitchAskPublicRequest' crates/` — MUST be empty before declaring a public request shape (if a public input struct is needed, reuse `PitchAskRequest`'s fields; do not introduce a near-duplicate).

## Fix (additive — no change to the authenticated path)
1. Add a new sub-module `routes_pitch_ask_public.rs` exposing
   `pub(super) async fn ask_public_response(req, store, peer_ip: &str) ->
   (&'static str, String)`. It:
   a. **Per-IP rate-limit FIRST** — reuse the existing limiter
      (`rate_limit_tenant`/`rate_limit`) keyed on `peer_ip`; over-budget →
      return the existing `429 Too Many Requests` shape. Do NOT hand-roll a new
      bucket type.
   b. **Input length cap** — reject a request whose `question` exceeds a tight
      ceiling (e.g. `MAX_PUBLIC_QUESTION_LEN = 600` chars) with `400` before any
      work; empty `question` already 400s via `ask_response`.
   c. **Force the public channel + token ceiling** — call
      `routes_pitch_ask::ask_response` but pin the channel to the public web
      widget variant (the `PitchChannel` already used for anonymous visitors per
      `routes_pitch_ask_llm.rs:232`), so the truth-boundary policy applied is the
      strict public one. The LLM-fallback path is already env-gated
      (`ARGO_PITCH_LLM_FALLBACK`) and auto-off without a provider key — keep that;
      a corpus miss returns the deterministic `MISS_FALLBACK` line, never a
      fabricated claim.
2. **Mount it in the pre-bearer lane**: in `preflight_dispatch`
   (`routes_dispatch_post.rs:188`), add a check for
   `POST /api/v1/pitch/ask/public` **before** the `require_identity` call (mirror
   how `routes_wizard::dispatch_post` is placed at line 198), so an
   unauthenticated visitor reaches it. The existing authenticated
   `/api/v1/pitch/ask ` match (line 252) is unchanged.
3. Add the new module to `mod.rs` (`mod routes_pitch_ask_public;`) and add the
   public route to `routes_catalog.rs` so it appears in the route inventory.
4. **Wire `PortiaEmbed.tsx`** to `POST /api/v1/pitch/ask/public` with the
   visitor's question; on `429`/network error/non-2xx, fall back GRACEFULLY to
   the existing canned answer already in the component (no broken UI). Keep the
   canned copy as the offline fallback.
5. Truth-boundary is PRESERVED end to end: only corpus-grounded answers or the
   `MISS_FALLBACK` line are ever returned — no new path that could emit an
   un-grounded claim. Do NOT weaken `TruthBoundaryPolicy`.

## Acceptance (truth-bounded)
1. Test (Rust, `crates/argo-cli`): an **anonymous** `POST /api/v1/pitch/ask/public`
   (no bearer) reaches the handler and returns a **grounded** answer for an
   in-corpus question (assert the body is a canonical hit, NOT a 401/403).
2. Test: the SAME request shape sent to the authenticated `/api/v1/pitch/ask`
   without a bearer still 401s (the public route is additive, the private gate
   is intact).
3. Test: **over the per-IP rate limit** → `429 Too Many Requests` (reuse the
   existing 429 assertion shape from `rate_limit_tenant_tests.rs`).
4. Test: an **oversized** `question` (> `MAX_PUBLIC_QUESTION_LEN`) → `400`,
   no LLM call.
5. Test: a question **outside the corpus** with the LLM path off →
   `MISS_FALLBACK` line, never a fabricated answer (truth-boundary holds).
6. `cargo test -p argo-cli` green; `web` build green with `PortiaEmbed` calling
   the live route; new files each < 500 LOC. Clippy clean on touched files
   (boundary-note the pre-existing `routes_settings_credentials` await-holding-lock).

## On ship (same commit)
Update the `FEATURE_MATRIX.md` row for the Pitch Agent / public web widget to
note the public rate-limited route is LIVE (status badge + commit SHA + route
`POST /api/v1/pitch/ask/public`). Do NOT regenerate `feature-matrix.html` (DEC-043,
gitignored). Note this makes the landing-page truth-boundary proof REAL.

## Effort
M — one new ~150-LOC handler reusing `ask_response` + the existing rate limiter,
one pre-bearer mount line, one `PortiaEmbed` wire-up + fallback, tests. Security
boundary is the load-bearing part: the route is anonymous, so per-IP limit +
input cap + truth-boundary preservation are non-negotiable.
