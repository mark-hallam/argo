# Mission load-bearing wire — richer VoiceProfile fields (tone/formality/lexicon)

**Owner**: claude. **Depends on**: `VoiceProfile` SHIPPED with `preamble` field (verified at `crates/argo-identity/src/types_role.rs:19`); `Persona.voice_profile` wired (line 35).
**Authored against**: origin/main `1f21d8ed` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-voice-profile-richer-fields`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #48 ("Build 'voice profile' per persona"). `VoiceProfile { preamble: String }` exists today, but a single freeform preamble is a weak abstraction — operators want structured tone/formality/lexicon controls so personas don't drift across model upgrades. Mission Claim 1 ("real teammate") needs persona voice to be a first-class, structured concept.

## Scope

Extend `VoiceProfile` with structured fields alongside the existing `preamble` (back-compat preserved):

```rust
pub struct VoiceProfile {
    pub preamble: String,                          // existing — freeform preamble
    #[serde(default)]
    pub tone: Tone,                                // NEW — high-level register
    #[serde(default)]
    pub formality: Formality,                      // NEW — 0-4 scale
    #[serde(default)]
    pub lexicon: Vec<String>,                      // NEW — domain vocabulary hints (e.g. "MTTR", "DAU")
    #[serde(default)]
    pub forbidden_phrases: Vec<String>,            // NEW — strings the voice must never use
}

pub enum Tone { Neutral, Warm, Direct, Playful, Formal }      // default Neutral
pub enum Formality { Casual, Conversational, Professional, Executive, Legal }  // default Conversational
```

NEW `VoiceProfile::compose_preamble() -> String` materializes a final preamble from the structured fields PLUS the freeform `preamble` (concatenated in deterministic order). The agent-prompt path calls `compose_preamble()` instead of reading `preamble` directly — this is the only behavioral change.

Back-compat: existing personas with just `{ preamble: "..." }` parse unchanged; `tone`/`formality`/`lexicon`/`forbidden_phrases` default to Neutral/Conversational/empty/empty. `compose_preamble()` returns the existing preamble verbatim when structured fields are at defaults.

This is NOT in scope: per-tenant voice templates; voice-grading metric (separate brief); model-side enforcement of `forbidden_phrases` (downstream concern); per-message voice override.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-identity/src/types_role.rs` — narrowly scoped: extend `VoiceProfile` with the 4 new fields; ADD `Tone` + `Formality` enums with serde + Default; ADD `compose_preamble() -> String` method. NO change to `Persona` struct (continues to hold `voice_profile: VoiceProfile`). NO removal of `preamble` field. File currently ~80 LOC — verify under 500 after additions.
- `crates/argo-identity/src/voice_profile_tests.rs` (NEW or append) — 8 unit tests: legacy-preamble-only-composes-verbatim; tone-warm-prepends-instruction; formality-executive-prepends-instruction; lexicon-appends-vocabulary-hints; forbidden-phrases-appends-negation; all-fields-compose-deterministic-order; default-tone-neutral; default-formality-conversational.
- `crates/argo-runtime/src/<wherever-agent-prompt-builds>.rs` (verify in first 5min via `rg -n "voice_profile\|preamble" crates/argo-runtime/`) — narrowly scoped: replace direct `.preamble` access with `.compose_preamble()` call. NO other change.

## Files you must NOT touch

- `crates/argo-identity/src/store.rs` — Persona storage is upstream
- `crates/argo-store/migrations/**` — `VoiceProfile` lives in serde JSON (Persona row), not a structured DB column → no migration needed. Verify in first 5min that Persona's serialized representation accepts new fields without a migration (it should, since serde-default handles it).
- Any non-identity/non-runtime crate

## Acceptance criteria

- [ ] `VoiceProfile` extended with `tone`, `formality`, `lexicon`, `forbidden_phrases` (all serde-default)
- [ ] `Tone` enum with 5 variants, `Default = Neutral`
- [ ] `Formality` enum with 5 variants, `Default = Conversational`
- [ ] `compose_preamble()` returns deterministic string composition
- [ ] Legacy personas (just `preamble`) parse + compose unchanged → byte-identical agent prompt
- [ ] Agent-prompt path calls `compose_preamble()`, not `.preamble`
- [ ] 8 unit tests
- [ ] All existing Persona/VoiceProfile tests still pass byte-identical
- [ ] No new public API outside the listed exports
- [ ] No DB migration (Persona JSON column accepts new fields via serde-default)
- [ ] `cargo build -p argo-identity -p argo-runtime` clean
- [ ] `cargo clippy -p argo-identity -p argo-runtime --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity -p argo-runtime` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-voice-profile-richer-fields`
- PR title: `identity: structured tone/formality/lexicon fields on VoiceProfile (mission-voice-profile-richer-fields)`

## Notes for the implementer

- `compose_preamble` deterministic order matters: structured-fields-first then freeform preamble, OR freeform first then structured. Pick ONE and document the choice in the module doc-comment. Operator overrides via freeform preamble should clearly "win" over structured defaults — recommend structured-first, freeform-last so freeform is the last instruction the model reads.
- Lexicon hints should be appended as "Vocabulary you may use: {comma-separated list}". Don't tell the model these are MANDATORY — they're hints.
- `forbidden_phrases` is best-effort: the model may still produce them. Real enforcement is in downstream output filtering (separate concern). The preamble instruction ("Never use these phrases: ...") gives the model a fair chance to comply.
- If the existing `voice_profile_tests.rs` file doesn't exist, create it sibling to types_role.rs.
