# V1.3 — Skill auction for handoff routing

**Owner**: claude. **Depends on**: claude-orchestrator-handoff-walker
(#825), claude-handoff-routing-rules (queued) merged.

## Scope

When a Session needs handoff but no specific worker is named
AND no routing rule matches, today the escalation walker
falls through to "human operator". A better fallback:
**workers bid for the work** based on their available skills.

Mechanism:
1. Handoff posts a "task description" to a per-tenant
   auction channel (in-process broadcast).
2. Active workers each compute a fit score: how many of the
   task's required-skills they have, weighted by skill
   TrustGrade and quality score (claude-skill-quality-judge
   feeds this).
3. Workers reply with `(worker_id, score, eta_estimate)`.
4. Auction closes after 5s; highest score wins; ties broken
   by lowest eta.
5. Winner gets the handoff; losers go on with their work.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-orchestrator/src/auction.rs` (NEW)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: re-export
- `crates/argo-orchestrator/src/handoff_walker.rs` —
  narrowly: insert auction step in fallback chain
- `crates/argo-store/migrations/<next>__handoff_auctions.sql`
  (NEW — auction_log + auction_bid tables for audit)
- `crates/argo-cli/src/handlers/handoff_auctions_*.rs` (3 NEW
  — list/show/cancel)
- `crates/argo-cli/src/cli_handoff_cmds.rs` — narrowly: add
  `Auctions` sub-subcommand variants

## Acceptance criteria
- [ ] Task description triggers auction broadcast
- [ ] Workers compute fit score from skill set + trust + quality
- [ ] Auction closes after configurable timeout (default 5s)
- [ ] Winner persists; losers logged (audit)
- [ ] No-bid case falls through to existing escalation
      (human operator) cleanly
- [ ] `argo handoff auctions list/show/cancel` work
- [ ] `--json` envelope universal
- [ ] 12 unit tests (scoring formula + tie-break + timeout) +
      1 integration test (3 workers bidding)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-skill-auction`
- PR title: `orchestrator: skill auction for handoff routing (claude-skill-auction)`
