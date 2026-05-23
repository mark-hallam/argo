# eval-marketplace-oci-registry-scan

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-marketplace-oci-registry-scan`.

🟡 **P2 CLAIM-4 — replace hardcoded MANIFEST_INDEX with real OCI registry catalog scan.** Source: MISSION_BACKLOG item #71.

## Scope

Audit `argo-marketplace` skill-marketplace surface. Current state: `MANIFEST_INDEX` is hardcoded. Should be a real OCI registry catalog scan against `github.com/mark-hallam/skycot-packs` (or wherever the operator-locked registry sits). Output: per-pack manifest fetch + signature verify + ingest into local skill catalog. Cross-tenant: each tenant gets its own ingested catalog view, not a workspace-global one.

## Parallelism

- Manager-worker allowed: no
- Max children: 0

## Files you may touch

- `crates/argo-marketplace/src/` — narrowly scoped: replace MANIFEST_INDEX consumer.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Acceptance criteria

- [ ] OCI registry catalog scan + per-pack ingestion.
- [ ] Cross-tenant negative.
- [ ] `cargo clippy -p argo-marketplace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `eval-marketplace-oci-registry-scan`
- PR title: `eval(marketplace): OCI registry catalog scan replaces MANIFEST_INDEX (eval-marketplace-oci-registry-scan)`

## Notes for the implementer

- ~200-400 LOC. Cite MISSION_BACKLOG #71.
