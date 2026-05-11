# Mission load-bearing wire — `published:verified:*` ingestion path with signature attestation

**Owner**: claude. **Depends on**: `argo-skill::sign::{load_public_key, read_signature_artifact, verify_skill_signature, SkillVerifyOutcome}` SHIPPED (verified via `crates/argo-cli/src/handlers/skill_verify_sig.rs`); `argo-cli::handlers::skill_verify_sig` SHIPPED; `mission-marketplace-oci-registry-scan` (sibling, in TODO — not a hard dep here).
**Authored against**: origin/main `f24073c3` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-publish-verified-ingest`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #74 — "Build `published:verified:*` ingestion path with signature attestation." Today `argo skill verify-sig` works as a manual one-shot, but the install pipeline does NOT auto-verify signed skills on pull from marketplace. Mission promise: when a skill carries a `signed` annotation/sidecar, the install path runs `verify_skill_signature` BEFORE writing to the local skill store; on verification failure the install aborts with a clear error.

## Scope

1. **NEW `SkillInstaller::install_verified` path**: extends existing `SkillStore::install` flow. Behavior:
   - Source is a marketplace pull (not a local-file install).
   - The pulled manifest carries an `org.skycot.signed = "true"` annotation OR a co-located `<skill>.sig` sidecar artifact in the registry.
   - Call `verify_skill_signature(registry, name, tag, &pubkey, payload)` from `argo-skill::sign`.
   - On `SkillVerifyOutcome::Ok` → continue install; mark stored `Skill.trust_grade` = `PublishedVerified` (existing enum variant).
   - On `Fail(reason)` → return `ArgoError::Capability("signature verification failed: {reason}")` — DO NOT install.
   - On `Missing` (no signature found) → behavior depends on `--require-signature` flag (see step 3).

2. **Public-key trust store**: NEW `~/.argo/trusted-publishers.toml` (operator-managed):
   ```toml
   [[publisher]]
   id = "skycot-official"
   pubkey_path = "~/.argo/keys/skycot-official.pem"
   ```
   Code path: NEW `argo-skill::trust_store::load_trusted_publishers() -> Vec<TrustedPublisher>`. Missing file → empty list (back-compat).

3. **NEW CLI flags on `argo skill install`**:
   - `--require-signature` — fail-fast if pulled skill has no signature; default OFF for back-compat.
   - `--publisher <id>` — pin to a specific trusted publisher; otherwise tries each in order.
   - `--insecure-skip-verify` — explicit operator opt-out, prints `tracing::warn!`; useful for dev.

4. **Audit row**: every install attempt writes an `argo_audit` row with `kind: "skill_install"`, `outcome: "verified" | "unverified-allowed" | "verification-failed"`. Reuses existing audit primitives.

5. **`argo skill install --json` envelope** extended with `signature: { verified: bool, publisher_id: Option<String>, fingerprint: Option<String> }`. Operator inspection.

This is NOT in scope: keyless verify via Sigstore Fulcio/Rekor (cosign keyless is a separate brief — `mission-skill-sigstore-keyless`); revocation lists; transparency log; key rotation tooling.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/trust_store.rs` (NEW, ≤ 200 LOC) — `TrustedPublisher` struct + TOML loader.
- `crates/argo-skill/src/trust_store_tests.rs` (NEW, sibling, ≤ 150 LOC) — 5 unit tests: missing-file-empty; parse-multi-publisher; bad-path-skip-with-warn; expanded-tilde-path; duplicate-id-error.
- `crates/argo-skill/src/install_verified.rs` (NEW, ≤ 250 LOC) — verified-install path wrapping `SkillStore::install`.
- `crates/argo-skill/src/install_verified_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: ok-installs-marks-trust-grade; fail-aborts-no-write; missing-without-flag-allows; missing-with-require-signature-fails; insecure-skip-warns-and-installs; audit-row-recorded.
- `crates/argo-skill/src/lib.rs` — narrowly scoped: re-export `trust_store::*` + `install_verified::*`.
- `crates/argo-cli/src/handlers/skill_install.rs` (verify path; may be `handlers/skill.rs::install`) — narrowly scoped: thread new flags + call `install_verified` when source is marketplace. ≤ 50 LOC delta.
- `crates/argo-cli/src/cli_subcommands_skill.rs` (verify path) — narrowly scoped: ADD 3 new flags on `Install` variant.

## Files you must NOT touch

- `crates/argo-skill/src/sign.rs` — sign/verify surface stable; reuse only
- `crates/argo-skill/src/store.rs::install` — wrap, don't modify
- `crates/argo-skill/src/trust.rs` — `TrustGrade` enum stable; reuse existing `PublishedVerified` variant
- `crates/argo-store/migrations/**` — no schema change
- Any non-skill/non-cli crate

## Acceptance criteria

- [ ] `~/.argo/trusted-publishers.toml` parses; missing file → empty list (no error)
- [ ] Marketplace-source install with signature + matching publisher → `PublishedVerified` skill installed
- [ ] Verification failure → install ABORTS with `Capability` error; no skill row written
- [ ] `--require-signature` flag makes missing-signature a hard error
- [ ] `--insecure-skip-verify` warns then installs anyway
- [ ] Audit row written for every install attempt with correct outcome
- [ ] JSON envelope includes `signature.{verified,publisher_id,fingerprint}`
- [ ] 5+6 unit tests + 1 integration smoke (verified-install golden path)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-skill -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-publish-verified-ingest`
- PR title: `skill: verified-install path with trusted-publisher trust store (mission-skill-publish-verified-ingest)`

## Notes for the implementer

- The `~/.argo/trusted-publishers.toml` is operator-managed; this brief just adds the parser. Bootstrap docs (which publishers to trust by default) is a follow-up.
- The fingerprint in JSON output should be sha256 of the DER-encoded public key — not the file path. Stable across operator config moves.
- Keyless (Sigstore Fulcio) verify is INTENTIONALLY out of scope. Don't add the dep.
- For "marketplace source" detection: piggyback on whatever existing `argo skill install` already uses to distinguish local-file from registry-pull. If unclear, document the detection logic in PR body.
- The `--insecure-skip-verify` warning is load-bearing — make it loud (red-text or explicit "INSECURE" prefix).
