# tg3b-capability-grant-write-containment-and-audit-hardening

**Owner**: claude or codex — fence-adjacent → conductor adversarial review before merge.
**Depends on**: tg3-durable-worker-capability-grant MERGED (#2834).
**Authored against**: origin/main `f9eb6bec3` on `2026-06-12` (against the #2834 PR head — re-verify line numbers post-merge).
**Validate before claim**: bash scripts/check-brief-freshness.sh tg3b-capability-grant-write-containment-and-audit-hardening

## Source (adversarial review of #2834, 2026-06-12 — CONFIRMED-SOUND with hardenings)

The tg3 review confirmed every load-bearing invariant holds. Two of its four
recommended hardenings were applied in-PR by the conductor (JWT-first
grantor + case-folded always-denied check). The remaining two are this brief —
both defense-in-depth, neither a reachable exploit under current invariants:

1. **Write-path containment (latent footgun, P2):** the grant route writes to
   `loaded.root` derived from `worker.vertical_id`; `is_genesis_pack`
   (`routes_worker_capability_grant.rs`, `is_genesis_pack`) is a substring
   check for a `.genesis` path component, NOT a containment check. Safe today
   only because every reachable persist path keeps `vertical_id`
   traversal-free (catalog hire 400s unknown ids; Genesis mints constrained
   `<archetype>-<suffix>` slugs; `WorkerUpdate::with_vertical_id` is unwired).
   If `vertical_id` ever becomes free text, a `..`-traversed path containing
   `.genesis` becomes a cross-tenant manifest WRITE. Fix: before `persist`,
   canonicalize `loaded.root` and assert it lives under
   `verticals/.genesis/<sanitize(authenticated_tenant)>/` — refuse otherwise
   (500/409, audited). Negative test: a crafted root outside the tenant
   subtree is refused.

2. **Audit write as hard prerequisite + dedicated kind (P3):** `write_audit`
   swallows sink errors with a `tracing::warn`, so a grant can persist and
   return 200 while its audit row silently fails — on a fence-loosening
   surface the audit row is the after-the-fact control. Also grant success
   uses generic `AuditKind::HttpRequest` (weak feed signal). Fix: fail the
   request (500, no grant persisted — order the audit write BEFORE persist,
   or roll back on sink failure) when the audit sink errors; introduce a
   dedicated audit kind for grant/revoke if the kind registry has LOC
   headroom (if not, keep HttpRequest and note why in the PR).

## May touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_capability_grant.rs` (+ tests)
- the audit-kind registry file (additive variant only, if headroom allows)

## Acceptance / verifier

1. Negative test: a `loaded.root` resolving outside the authenticated
   tenant's `.genesis` subtree is refused before any write.
2. Test: when the audit sink errors (test seam / failing sink), the grant
   request fails and the manifest on disk is unchanged.
3. Existing 9-test suite still green; `cargo clippy -p argo-cli` clean.

~60–120 LOC. **🟡 P2 — defense-in-depth on the fence-loosening surface.**
