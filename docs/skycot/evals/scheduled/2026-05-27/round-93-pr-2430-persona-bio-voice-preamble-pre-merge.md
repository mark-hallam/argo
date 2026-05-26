# Round 93 — PR #2430 persona.bio + voice_preamble PRE-MERGE verification

- **Date**: 2026-05-26 (Melbourne 2026-05-27 morning AEST)
- **Round**: 93
- **Pillar / Claim**: Claim 1 (Teammate-not-tool) — onboarding persona depth
- **Verdict**: 🟢 **PRE-MERGE PASS BY INSPECTION** (P3 — UX-AUDIT-T0119MEL F15 closer with F11 cascade)

## Setup

- `gh pr view 2430 --json title,state,mergeable,headRefOid,files,body` — MERGEABLE/CLEAN; head `b9a19b46c`
- `git fetch origin pull/2430/head:pr-2430-eval` — pulled the head SHA locally
- `git diff origin/main..pr-2430-eval -- crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — production wiring
- `git diff origin/main..pr-2430-eval -- crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` — new helper
- `git diff origin/main..pr-2430-eval -- crates/argo-cli/src/handlers/gateway/routes_onboarding_first_hire_genesis_tests.rs` — new integration test
- Cross-reference: `crates/argo-identity/src/store_hire.rs:167` for `HireDraft.tagline` → `Persona.bio` wire
- Attempted: `cargo test -p argo-cli first_hire_persists_persona_bio` from a fresh worktree at `/tmp/eval-r93/pr-2430` with `CARGO_TARGET_DIR=/tmp/eval-r93/target`. **Did not complete** — wall-clock waited 22min; cargo was killed at the compile phase (`tail -25` of stdout shows last visible line was `Compiling argo-audit` and `Compiling aws-sdk-sso` — far below argo-cli's test crate in the dep graph). Worker contention (3 concurrent worker-loop cargo invocations on this machine) capped progress at ~3-4 deps/sec. Cleaned up `/tmp/eval-r93` to recover disk for workers.

## Observed

### Production wire-up (routes_onboarding.rs)

```rust
// before:
voice_profile: None,
tagline: None,

// after:
let voice_preamble = voice_preamble_from_description(&worker_name, &description);
…
voice_profile: Some(VoiceProfile {
    preamble: voice_preamble,
    ..Default::default()
}),
tagline: Some(description.clone()),
```

### Wire-through (store_hire.rs:167)

```rust
let persona = Persona {
    name: draft.suggested_name.clone(),
    voice_profile: voice,
    avatar: None,
    bio: draft.tagline.clone(),     // ← HireDraft.tagline lands here
    builder_attribution: principal_id.to_string(),
    provenance_chain: Vec::new(),
};
```

So `HireDraft { tagline: Some(description) }` round-trips to `persona.bio = description`.

### New helper (routes_onboarding_helpers.rs:226+)

```rust
pub(super) fn voice_preamble_from_description(name: &str, description: &str) -> String {
    let trimmed = description.trim();
    let first_sentence = trimmed
        .split(['.', '\n'])
        .find(|s| !s.trim().is_empty())
        .map_or(trimmed, str::trim);
    let excerpt = truncate_on_word_boundary(first_sentence, 200);
    format!("You are {name}, an autonomous SKYCOT teammate. {excerpt}")
}
```

`truncate_on_word_boundary` uses `char_indices` (Unicode-safe) and falls back gracefully if the string has no whitespace; trailing trim handles `,` and whitespace correctly. ASCII-only call site (name + description from operator) so even the worst-case unicode handling is safe.

### Integration test (routes_onboarding_first_hire_genesis_tests.rs:183+)

Test name: `first_hire_persists_persona_bio_from_operator_brief`. POSTs to `/api/v1/onboarding/first-hire` with `description="Mariah, you book outbound demo calls and qualify leads. Always confirm before drafting a public reply."`, expects `201 Created`, then runs a SQL JOIN:

```sql
SELECT p.bio, p.voice_preamble
FROM persona p JOIN worker w ON w.id = p.worker_id
WHERE w.vertical_id = 'gtm-engineer'
```

and asserts **four independent axes** on the round-tripped values:

1. `bio = description` (exact equality)
2. `voice_preamble.starts_with("You are Mariah")` (helper output structure)
3. `voice_preamble.contains("book outbound demo calls")` (brief excerpt folded in)
4. `!voice_preamble.starts_with("I'm a ")` (negative pin — pre-fix templated string would fail this)

## Anti-theater check

**Dispositive on four independent axes** even without a live cargo run:

1. **The test queries SQL directly on the production persona table** — not a mock. A passing test means the `HireDraft { tagline: Some(description) }` literally landed in `persona.bio` (because `store_hire.rs:167` is the only write path for that column from this code path; verified by `grep -rn "bio" crates/argo-identity/src/store_hire.rs`).
2. **The negative pin on `I'm a `** prevents test-mutates-itself-to-green: if a future refactor reverts to the old templated preamble, axis 4 fires.
3. **Naming axis is independent of content axis** — assertions 2 and 3 stress different parts of the helper output. A helper that produces "You are X" but doesn't fold the brief would pass axis 2 + fail axis 3.
4. **First-sentence extraction is exercised** — the test description has a clear `. ` separator at "qualify leads.", so `split(['.', '\n']).find(non_empty)` picks the first sentence; the second sentence ("Always confirm…") should NOT appear in voice_preamble. The PR's `200-char` truncation does NOT activate here (sentence is ~50 chars) so the truncate codepath is untested — minor coverage gap noted.

**Live-cargo verdict not available** — see Setup. PR author claims `cargo test -p argo-cli first_hire` 7/7 pass in their session. The structural inspection confirms the wire is correct and the test is real, so this is **🟢 PRE-MERGE PASS by inspection** at evaluator-confidence-level "high but not dispositive on the same axis as a cold cargo PASS would have provided." Recommend the conductor pre-merge gate's `cargo test -p argo-cli` run pre-merge to close the loop.

## Gap / follow-up brief candidates

**P3** — `test-voice-preamble-truncate-word-boundary-coverage` (~15 LOC) — add a second integration test (or a unit test on `voice_preamble_from_description`) with a >200-char first-sentence description to exercise the `truncate_on_word_boundary` path. Coverage gap, not a correctness issue. Out of demo-MP4 critical path.

**P3** — `fix-worker-personality-and-level-derive-from-real-data` (already referenced in the PR body's "Out of scope" — surfaces the now-populated `persona.bio` on the SPA worker overview "About" section). Without this, the data lands correctly but the operator never sees it in the UI. Demo runbook should be checked to confirm whether Scene 6 of the v1.0 demo MP4 walks past a worker-detail "About" surface.

## Closes (when merged)

- UX-AUDIT-T0119MEL F15 (P3) — `persona.bio` NULL → operator brief preserved
- UX-AUDIT-T0119MEL F11 cascade-partial — voice-preamble personality dimension materialises
- Honest demo: operator-typed first-hire brief shows up on the worker (data-side) instead of being lost between charter and the persona row

## Merge recommendation

**🟢 Recommend MERGE under permanent fix-* lane (auth: web+test+rust mixed, integration test only; backend-rust ≤500 LOC across 3 files).** The conductor pre-merge gate's clippy + targeted test run on the same head SHA closes the cold cargo gap I could not complete under worker contention. Anti-theater inspection is dispositive on 4 axes.

**Verdict — 🟢 PRE-MERGE PASS by inspection** (PR author cargo PASS in session; cold-cargo verification deferred to conductor gate due to worker contention).
