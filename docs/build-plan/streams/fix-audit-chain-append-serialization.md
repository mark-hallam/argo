# fix-audit-chain-append-serialization

Authored against: origin/main c359fc5ca on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-audit-chain-append-serialization

## Goal

Funnel every audit-row append through the serialized canonical writer so two
concurrent appends can never fork the hash chain. The hash-chained,
tamper-evident audit log is differentiator #1 in the marketing pillars — the
chain must be linear under concurrency, not just correctly hashed.

## Why now (security audit 2026-06-10, P2 — core-promise hardening)

- The canonical sink serializes appends in one transaction
  (`crates/argo-audit/src/sink_chain.rs:161-195`).
- ~12 modules instead do read-`last_global_hash`-then-`INSERT` OUTSIDE that
  transaction. All DO chain-hash (no unhashed side-doors found), but two
  concurrent appends can read the same `parent_hash` → sibling rows = a
  forked chain.
- Confirmed sites: `routes_tenant_theme.rs:272-296`, `hipaa_startup_check.rs:70`,
  `worker_promote.rs:394`, `security_rotate_keys.rs:414`, `gdpr_delete.rs:254`,
  `tenant_transfer.rs:103`, `orchestrator/walker_helpers.rs:189`,
  `compliance/delete.rs`, `compliance/export.rs` (+3 more — re-enumerate with
  `rg 'last_global_hash' crates/ -l` at claim time).

## Fix

Route the side-door appends through `AuditSink` / `sink_tx::write_in_tx`
(within a transaction) or, where a sink handle genuinely can't reach, take
the same lock/transaction the sink uses. NO new append API — consolidate on
the canonical writer. Mechanical per-site change; keep each site's AuditKind
and payload byte-identical.

## May touch

- the ~12 enumerated call sites
- `crates/argo-audit/src/sink_chain.rs` / `sink_tx.rs` (only if a small
  `pub(crate)` helper is needed for store-handle call sites)
- tests for the touched sites

## Acceptance / verifier

1. Concurrency regression test: N parallel appends (mixed canonical +
   formerly-side-door paths) → chain verifies linear (existing chain-verify
   helper), zero forks (no two rows share a parent_hash).
2. `rg 'last_global_hash' crates/` shows reads ONLY inside the canonical
   writer/verify modules.
3. Existing audit round-trip + chain-verify tests green.

## Size / owner

~12 sites × small mechanical change + 1 concurrency test. Owner: claude.
🟡 P2 but core product promise — schedule ahead of polish work.
