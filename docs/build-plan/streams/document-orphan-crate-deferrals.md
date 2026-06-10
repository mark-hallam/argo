# document-orphan-crate-deferrals

Authored against: origin/main 75cc69d086eaf81301ce6909f6acd74dcd413646 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh document-orphan-crate-deferrals

## Goal

Record the deferral rationale for the remaining orphan crates (zero workspace
consumers) so the audit's "no recorded reason" finding is closed — the same
treatment DEC-047 gave argo-acp.

## Why now (architecture audit 2026-06-10, P2-6)

Orphan crates with zero reverse-deps, behind `zero-caller-allowed` markers with
thin/no rationale: `argo-mcp-connector-onedrive` (~1047 LOC),
`argo-skill-sdk` (~730 LOC). (argo-acp already documented via DEC-047;
test-cassette + vertical-regression have script consumers.)

## Fix

For each: enrich the `zero-caller-allowed` marker comment in its Cargo.toml with
the deferral reason + a pointer:
- `argo-mcp-connector-onedrive`: DEFERRED-v1.x per DEC-001 (enterprise connector
  stubs kept, not v1-polished). Reference DEC-001.
- `argo-skill-sdk`: consumed by EXTERNAL plugin authors (not a workspace
  consumer by design) — document that on the marker so it's not mistaken for
  dead code.
Zero-caller lint must stay green (markers preserved). No code deletion.

## May touch

- crates/argo-mcp-connector-onedrive/Cargo.toml
- crates/argo-skill-sdk/Cargo.toml

## Acceptance / verifier

1. `bash scripts/check-zero-caller-substrate.sh` still green.
2. Each marker carries a one-line reason + DEC/consumer reference.

## Size / owner

Doc/comment-only. Owner: claude (codex-eligible). 🟢 P3 — closes the audit's
"orphan crate, no recorded reason" finding.
