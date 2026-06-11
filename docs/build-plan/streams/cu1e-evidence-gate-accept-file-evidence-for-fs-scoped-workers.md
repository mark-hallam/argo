# cu1e-evidence-gate-accept-file-evidence-for-fs-scoped-workers

**Owner**: claude or codex — product/safety-policy change → conductor review before merge.
**Depends on**: cu1 (#2812) + cu1b (#2815) merged; cu1d (#2819, NetEgress for web tools) is the *workaround sibling*, not a dependency.
**Authored against**: origin/main `ea2350bfd` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu1e-evidence-gate-accept-file-evidence-for-fs-scoped-workers

## Problem (found live 2026-06-11 staging the cu5 approve→execute test)

The #729 evidence gate hard-codes "evidence = web research":
`work_artifacts/evidence.rs:56` — `is_sufficient() = total_search_hits >= 1 && web_extract_total_chars >= 200`.
When insufficient, `worker_actions_emit.rs:415` downgrades every approval-gated
proposal from `pending` → `blocked`.

Consequence: a worker without web tools can **never** land a held action in
`/approvals` — its approval-gated proposals are all evidence-blocked, no matter
how strong its actual evidence is. Live proof: Chase (AR teammate,
`fs_root=~/SkycotDemo/Inbox`, file tools granted) read the real
Globex invoice with `file_read` and proposed `send_payment_reminder` — the
invoice file IS the evidence — yet the proposal could only land `blocked`
(web_search was additionally rbac-denied pre-cu1d; even post-cu1d, forcing an
AR clerk to do a web search to "prove" an invoice it just read is theater, the
exact thing DEC-029 bans).

## Fix direction (pick at implementation, smallest honest change wins)

Option A (preferred): count `file_read` results from inside the worker's
`fs_root` toward sufficiency — e.g. `evidence_sufficient = web-sufficient OR
(fs_read_calls >= 1 && fs_read_total_chars >= 200 from fs_root paths)`. The
work-artifacts evidence counters already see tool results; extend the counter
struct + `is_sufficient()`.

Option B: per-manifest evidence policy — `[actions.<name>].evidence_policy =
"web" | "filesystem" | "any" | "none"` defaulting to today's `"web"` so
existing verticals are unchanged. More flexible, more surface.

Either way: the gate's PURPOSE stays — an approval-gated action with *zero*
grounding still lands `blocked`. The change only recognizes evidence types
that match what the worker is fenced to.

## May touch

- `crates/argo-cli/src/handlers/work_artifacts/evidence.rs` (+ tests)
- `crates/argo-cli/src/handlers/work_artifacts/mod.rs` (counter plumb)
- `crates/argo-cli/src/handlers/worker_actions_emit.rs` (gate message)
- (Option B only) `crates/argo-vertical` manifest schema + validation

## Acceptance / verifier

1. Unit: a turn with 1× in-fence `file_read` ≥200 chars + 0 web calls →
   approval-gated emission lands `pending` (Option A) — plus regression pin:
   0 reads + 0 web → still `blocked`.
2. Live: Chase Run-now proposing `send_payment_reminder` grounded ONLY in the
   invoice file → row appears in `GET /api/v1/approvals` (pending,
   requires_approval) → cu5 approve→execute works without any web grant.
3. `cargo test -p argo-cli` (worker_actions_* + work_artifacts) green; clippy.

## Size / priority

~80-150 LOC incl. tests. **🟡 P2 — DEC-050 demo-path quality** (cu1d makes web
evidence reachable as a workaround; this makes the gate honest for fs-scoped
workers). NO THEATER alignment: evidence should be the thing the worker
actually worked from.
