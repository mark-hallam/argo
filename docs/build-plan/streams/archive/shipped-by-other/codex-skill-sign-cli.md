# V1.4 — Skill signing CLI

**Owner**: codex. **Depends on**: codex-vertical-publish-real-oci
(#908) merged.

## Scope

#908 landed real OCI Distribution Spec push/pull via the
`oci-distribution` crate. Verticals can now go to ttl.sh / ghcr.io
as real OCI artifacts. Missing: cryptographic provenance —
right now any operator can publish anything and downstream pulls
have no integrity check beyond manifest sha256.

Add a thin signing layer using **cosign-compatible** signatures
(detached, stored as a sibling OCI artifact per the cosign spec):

- `argo skill sign <name> [--key <path>] [--keyless] [--json]` —
  sign a local skill manifest. `--key` uses a local cosign-format
  ed25519 keypair; `--keyless` uses Sigstore Fulcio (deferred to
  follow-up — for v1, only `--key` is required).
- `argo skill verify <ref> [--key <pubkey>] [--registry <ref>]
  [--json]` — verify a published skill against its signature.
  Returns OK / FAIL / MISSING signature.
- `argo skill keygen [--out <dir>]` — generate a fresh cosign
  ed25519 keypair (private + public) for testing/onboarding.

Reuses the OCI registry path from #908; signature lands as a
separate OCI artifact at the standard cosign sibling tag
(`<digest>.sig`).

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/skill_sign.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_verify.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_keygen.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_skill_search_cmds.rs` (or sibling) —
  narrowly: add `Sign`, `Verify`, `Keygen` variants
- `crates/argo-skill/src/sign.rs` (NEW) — pure signing logic
  (sign_payload / verify_payload / generate_keypair) using
  the existing `ed25519-dalek` crate (already a transitive dep)
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/skill_sign_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo skill keygen --out /tmp/keys` writes ed25519 keypair files
- [ ] `argo skill sign foo --key /tmp/keys/cosign.key` creates a
      detached signature artifact
- [ ] `argo skill verify foo:0.1.0 --key /tmp/keys/cosign.pub` returns OK
- [ ] Tampering with the manifest causes verify to return FAIL
- [ ] Missing signature returns MISSING (not error)
- [ ] `--keyless` is acceptable as deferred-to-follow-up note (not required for v1)
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 6 unit tests + 1 CLI smoke (sign → tamper → verify FAIL)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-skill-sign-cli`
- PR title: `skill: argo skill sign/verify/keygen — cosign-compatible signing (codex-skill-sign-cli)`
