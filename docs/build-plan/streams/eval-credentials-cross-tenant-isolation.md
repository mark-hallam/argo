# eval-credentials-cross-tenant-isolation

**Owner**: claude (evaluator-shape brief — investigation + findings + tests, NOT a runtime fix).
**Depends on**: none.
**Authored against**: origin/main `5a8726465` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-credentials-cross-tenant-isolation`.

🟡 **P1 — SECOND-HIGHEST UNAUDITED SURFACE.** Source: evaluator round-8 + auditor T21-39 §4 P1 #2:

> "`argo-credentials` stores OAuth + API keys; cross-tenant leak here breaks every customer's third-party tokens. Pattern from #2068 6-handler existence-leak + #2051 audit-export tenant-scope: cross-tenant work has been shipping P1-level value per audit, and credentials is unaudited so the surface is unknown."

`argo-credentials` is the crate that holds third-party authentication tokens for every connector / integration / channel SKYCOT touches:
- MCP connector OAuth refresh tokens
- Channel API keys (Slack, Twilio, Discord, etc.)
- Provider API tokens (Anthropic, OpenAI)
- Operator-provided service credentials

A cross-tenant leak here is **catastrophic**: tenant_a's Slack token granting tenant_b's worker access to tenant_a's Slack workspace. This surface has never been audited.

## Scope

Investigation brief, same shape as `eval-argo-agent-runner-turn-lifecycle`. Deliverable: comprehensive report at `docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md`.

### 1. Surface inventory

Walk every `pub` symbol in `crates/argo-credentials/src/`. Document:
- Storage primitives (CRUD on `credentials` table or equivalent)
- Resolver primitives (load-by-id, load-by-tenant)
- Refresh primitives (OAuth token refresh, key rotation)
- Encryption-at-rest: what's encrypted, with what key

### 2. SQL tenancy audit

For EVERY query in the crate:
- Does it bind `tenant_id`?
- Is it scoped via a tenant-bound primary key (e.g. `credential_id` UUID that's itself tenant-stamped)?

This is the same shape as the cross-tenant existence-leak fix in #2068 (worker handlers) but at the credentials layer.

### 3. Encryption-at-rest verification

- Is each credential encrypted before write?
- What's the key derivation? Per-tenant key, or workspace-wide key?
- If workspace-wide: is the storage layer + the API surface tenant-bound to prevent tenant_b from issuing a "give me credential X" against tenant_a's tagged row?

### 4. API surface audit

Walk every callsite of `argo-credentials` from `argo-cli`, `argo-channels`, `argo-mcp-*`, `argo-agent-runner`:
- Does the caller pass `tenant_id` correctly?
- Are there any callsites that omit tenant scoping (e.g. "load this credential by id without knowing whose")?
- Cross-tenant negative test for each consumer path.

### 5. OAuth refresh flow

OAuth refresh tokens are particularly sensitive because they grant long-lived access. Walk the refresh flow:
- Who triggers a refresh?
- Where does the new access token land?
- Is it tenant-bound on write-back?
- Is the refresh atomic with audit emission?

### 6. Rotation + revocation

- How are credentials rotated? (operator action vs auto?)
- How are they revoked? (delete row vs mark inactive?)
- Does revocation propagate to in-flight worker turns immediately, or only on next turn boot?

### 7. Decryption boundary

When a worker turn needs the actual token text, where does the decrypt happen?
- In `argo-credentials` itself (token never leaves the crate)
- In `argo-runtime` / `argo-channels` (token flows out as a `Secret<String>` newtype)
- Inline at the callsite (token flows out as a raw `String`)

Decryption-boundary leaks are common — verify the contract.

## Deliverables

ONE comprehensive report at `docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md` — 1500-3000 words, sections per §1-7. Each finding includes severity + file:line + brief-shape recommendation.

Optionally: `#[ignore]`'d FINDINGS-ONLY cross-tenant negative tests for any surface that lacks current coverage.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md` — the report.
- `OPTIONAL NEW: crates/argo-credentials/tests/eval_findings_*.rs` — `#[ignore]`'d FINDINGS-ONLY tests.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — append-only updates marking audited rows 🟢.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-credentials/src/**` — strict read-only. Audit, not refactor.
- Other crates outside the audit chain.

## Acceptance criteria

- [ ] Report covers all 7 sections.
- [ ] Every finding has severity + file:line + brief-shape recommendation.
- [ ] At least 1 cross-tenant negative test added as `#[ignore]`'d FINDINGS-ONLY (or note explicitly if surface is proven tenant-scoped).
- [ ] `mission-vs-code-gap-manifest.md` updated: argo-credentials row 🔴 → 🟢.
- [ ] If ANY P0 surface (cross-tenant token leak) is found, surface it in the PR body as a 🚨 ESCALATION + name the fix-brief.
- [ ] Cost target: 50-100K tokens.
- [ ] BUILD_STATUS.md updated with the report path.

## Branch + PR

- Branch: `eval-credentials-cross-tenant-isolation`
- PR title: `eval(credentials): cross-tenant isolation audit — 7-section findings (eval-credentials-cross-tenant-isolation)`

## Notes for the implementer

- ~150-400 LOC of report + optional FINDINGS-ONLY tests.
- **Highest leverage area**: §3 (encryption-at-rest) + §4 (API surface) + §7 (decryption boundary). Spend time on these. §1, §2, §6 are mechanical inventories.
- **Anti-theater**: if you can't construct a cross-tenant negative test, document why — but the default should be "we have a test."
- After this lands, the conductor queues fix-briefs from the recommendations.
- Cite auditor T21-39 §4 P1 #2 + evaluator round-8 manifest verbatim in PR body.
- This is pair-evaluator brief with `eval-argo-agent-runner-turn-lifecycle` — same investigation shape, different surface. Can be claimed in parallel by different workers.
