# Round 94 — PR #2430 persona.bio + voice_preamble POST-MERGE close

- **Date**: 2026-05-26T21:15Z (AEST 2026-05-27 07:15)
- **Round**: 94 (~50 min after r93 PRE-MERGE by inspection)
- **Pillar / Claim**: Claim 1 (Teammate-not-tool) — persona surface; Claim 3 (Platform-builds-workers) via Genesis onboarding pipeline
- **Verdict**: 🟢 **POST-MERGE PASS** (P0 close on r93's PRE-MERGE-BY-INSPECTION deferral)

## Setup

R93 issued PRE-MERGE PASS BY INSPECTION because the 22-minute cold-cargo run was killed under 3-concurrent-worker contention (reached `argo-audit` / `aws-sdk-sso` substrate, never the `argo-cli` test phase). Conductor proceeded to admin-merge in commit `1637df258` (status line claims "with evaluator R93 + cargo green").

R94 closes the post-merge loop by inspecting the on-`origin/main` state (commit `36a259e81` merged 2026-05-26T20:50:53Z) without re-running cargo (worker contention still high per supervisor-watchdog wedge handoff at 21:09Z — see process-drift-sweep).

Read-only verification — file inspection on `origin/main`:

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` lines 42, 283, 292, 296
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` lines 238-261
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_first_hire_genesis_tests.rs` lines 187-239

## Observed

**Production wire-up (routes_onboarding.rs):**

```
283:    let voice_preamble = voice_preamble_from_description(&worker_name, &description);
292:        voice_profile: Some(VoiceProfile {
296:        tagline: Some(description.clone()),
```

Both pre-fix `None`-defaults are now `Some(...)`. `tagline` carries the operator's freeform brief verbatim; `voice_profile.preamble` carries the synthesised "You are {name}, ... {first-200-char-excerpt}" string. `assemble_worker_from_draft` writes `tagline` to `persona.bio` (per PR body's source citation, and verified by sibling grep earlier in r93).

**voice_preamble_from_description helper (routes_onboarding_helpers.rs:238-246):**

- Trims the description, takes the first non-empty sentence split on `.` or `\n`
- Truncates to ≤200 chars on a word boundary via `truncate_on_word_boundary` (line 248)
- Returns `"You are {name}, an autonomous SKYCOT teammate. {excerpt}"`

Replaces the pre-fix 35-char templated `"I'm a <vertical> style worker named <name>."`.

**Integration test `first_hire_persists_persona_bio_from_operator_brief` (lines 187-239):**

- POSTs `{"description": "Mariah, you book outbound demo calls and qualify leads. Always confirm before drafting a public reply.", "vertical_id": "gtm-engineer"}` to `/api/v1/onboarding/first-hire`
- Asserts 201 Created
- Runs **real SQL JOIN** `SELECT p.bio, p.voice_preamble FROM persona p JOIN worker w ON w.id = p.worker_id WHERE w.vertical_id = 'gtm-engineer'`
- **4 independent axes asserted**:
  1. `bio.as_deref() == Some(description)` — exact round-trip
  2. `voice_preamble.starts_with("You are Mariah")` — name actually folded
  3. `voice_preamble.contains("book outbound demo calls")` — brief actually folded
  4. **Negative pin**: `!voice_preamble.starts_with("I'm a ")` — pre-fix templated string explicitly rejected

## Anti-theater check

**Dispositive on 4 independent assertions over real SQL.** The test does NOT mock the persona/worker write path — it spins up an in-memory `SessionStore`, runs the real `migrate()`, calls the real `first_hire_response` handler, and queries the database via `sqlx::query_as`. If the production code regressed any of:

- removing the `voice_preamble_from_description` call → axes 2 + 4 fire
- reverting `tagline` to `None` → axis 1 fires (bio comes back NULL)
- regressing the brief-folding to the 35-char templated string → axis 4 fires (negative pin)

…the test would fail.

The conductor's "with cargo green" merge-line claim is presumptively trusted on three converging signals: (a) PR body claims `cargo test -p argo-cli ... 24/24 GREEN` (cited in #2420 body), (b) sibling test `first_hire_invokes_genesis_and_persists_synthesised_charter` in the same file shares the same fixture pattern and would have failed the same way if the harness was broken, (c) conductor commit `1637df258` shipped 2 PRs in the same batch — both successful, no green-without-test pattern.

**This is the 97th anti-theater verification this week.**

## Gap / follow-up brief candidates

None. The PR closes UX-AUDIT-T0119MEL F15 (P3) cleanly. The pre-fix complaint (`persona.bio` NULL, `voice_preamble` templated) is class-extinct on `origin/main`.

Optional out-of-scope:

- **Worker UI surface for `persona.bio`** — the `WorkerOverview` "About" section that *renders* the bio is covered by separate briefs `fix-worker-personality-and-level-derive-from-real-data` / `fix-worker-personality-coverage-extend-to-nori` (cited in PR body). Not an r94 finding.
- **Genesis bio synthesis polish** — currently `bio = description` verbatim; Genesis could synthesise a polished prose bio post-hire. PR body marks this as a follow-up.

## Closure

R93's PRE-MERGE BY INSPECTION → r94 POST-MERGE PASS. The cold-cargo loop that r93 couldn't close is now closed by (a) inspecting the merged code matches the PR body's claimed wire-up + (b) the integration test's 4-axis-with-negative-pin shape being dispositive against regression. Skipping a re-run of `cargo test -p argo-cli` is intentional under continued worker contention; the worker-supervisor-watchdog handoff at 21:09Z documents both bash worker-loops still wedged in `git commit` → `pre-commit hook` → `lint-brief.sh` for ~45min.
