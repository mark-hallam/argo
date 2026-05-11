# Conductor advisory — Track E foundation + slice E1 (7 briefs)

Authored 2026-05-11 against origin/main `0cc3fdba` after the
post-E1 checkpoint sign-off. Paste the fenced block below into
the running conductor at the next tick. The conductor inserts 7
new TODO rows in BUILD_STATUS.md; workers can begin claiming E0
immediately; E1-a through E1-f wait until E0 MERGES (it's a
CONTRACT PR).

## Paste this to the conductor

```
MID-TICK ADVISORY — Track E foundation + slice E1 briefs queued.
Seven new TODO rows for the ledger. E0 is a CONTRACT PR; the
other six wait on it.

Documents changed:
  - docs/build-plan/PRODUCT_ROADMAP.md — Track E flipped
    NOT STARTED → IN PROGRESS.
  - docs/build-plan/streams/E0-frontend-foundation.md (NEW)
  - docs/build-plan/streams/E1-a-approvals-gateway-wireup.md (NEW)
  - docs/build-plan/streams/E1-b-approvals-filter-popover.md (NEW)
  - docs/build-plan/streams/E1-c-approvals-notifications-popover.md (NEW)
  - docs/build-plan/streams/E1-d-approvals-batch-decide.md (NEW)
  - docs/build-plan/streams/E1-e-approvals-pii-redaction.md (NEW)
  - docs/build-plan/streams/E1-f-approvals-websocket-live.md (NEW)

Action required from you this tick:
  Insert seven new TODO rows in BUILD_STATUS.md per the briefs
  listed. Commit ONE row per commit per the existing convention.

Row 1 — CONTRACT PR; downstream E1 rows wait on it.
  brief id:  E0-frontend-foundation
  owner:     claude
  status:    TODO
  PR col:    —
  note:      "**Track E FOUNDATION (CONTRACT PR).** Locks
              Next.js route groups ((app)/(hire)), NextAuth
              v5 bridging ARGO_GATEWAY_BEARER_TOKEN, AppShell +
              HireShell composites, typed gateway client with
              ApiError envelope, query-key conventions for
              downstream E1-E5 hooks. Existing approvals-inbox
              demo moves under (app)/ unchanged; mock fallback
              preserved when gateway returns 404 in dev.
              Downstream E1-a..f BLOCKED on this MERGE. Brief:
              streams/E0-frontend-foundation.md."

Row 2 — Track E slice E1 (approvals inbox).
  brief id:  E1-a-approvals-gateway-wireup
  owner:     claude
  status:    TODO  (BLOCKED until E0 MERGES)
  PR col:    —
  note:      "**Track E slice E1 (approvals inbox), brief 1/6.**
              Replaces mock data path in web/lib/api/approvals.ts
              with real gateway fetches via the E0 client wrapper;
              preserves SAMPLE_APPROVALS fallback when gateway
              returns 404 or is unreachable. Adds dev-only
              DataSourceChip (live / mock-offline / mock-auth-
              bypass). Brief:
              streams/E1-a-approvals-gateway-wireup.md."

Row 3
  brief id:  E1-b-approvals-filter-popover
  owner:     claude
  status:    TODO  (BLOCKED until E0 + E1-a MERGE)
  PR col:    —
  note:      "**Track E slice E1, brief 2/6.** Lights up the
              Filter button with a Radix Popover for worker /
              channel / status multi-select; URL-persistent state
              (?worker=...&channel=...&status=...) + inline
              FilterChipRail with × removers. Composes with the
              existing search query. Brief:
              streams/E1-b-approvals-filter-popover.md."

Row 4
  brief id:  E1-c-approvals-notifications-popover
  owner:     claude
  status:    TODO  (BLOCKED until E0 MERGES; sibling of E1-a..f)
  PR col:    —
  note:      "**Track E slice E1, brief 3/6.** Bell button opens
              a popover showing system-level notifications
              (budget, skill promotions, OAuth re-auth, drift,
              duty outputs, autonomy suggestions, incidents)
              grouped Today/Yesterday/Earlier. Per-row dismiss +
              optional inline CTA. Unread dot on bell. 60s
              polling when tab focused. Brief:
              streams/E1-c-approvals-notifications-popover.md."

Row 5
  brief id:  E1-d-approvals-batch-decide
  owner:     claude
  status:    TODO  (BLOCKED until E0 + E1-a MERGE)
  PR col:    —
  note:      "**Track E slice E1, brief 4/6.** Fuzzy rule-based
              similarity grouping (worker + action_type +
              normalized-detail hash) surfaces a BatchActionBar
              above groups of 3+ similar cards. Approve-all
              fires N parallel mutations with per-card rollback;
              review-each opens a swipe carousel. Refuse-all
              intentionally NOT offered per
              journey-approve-on-mobile.md's over-approval-risk
              guidance. Brief:
              streams/E1-d-approvals-batch-decide.md."

Row 6 — OPERATOR-PENDING QUESTIONS flagged in brief.
  brief id:  E1-e-approvals-pii-redaction
  owner:     claude
  status:    TODO  (BLOCKED until E0 + E1-a MERGE; **5 schema
                    open questions answered defensively with
                    conservative defaults — operator may revise
                    any of them in-place before claim**)
  PR col:    —
  note:      "**Track E slice E1, brief 5/6.** Client-side
              defense-in-depth PII layer masking pattern-
              matchable categories (email, phone, Luhn-valid
              card, IBAN, NZ IRD) at render time with RBAC-aware
              unmask + audit-on-elevated-view-request. Companion
              server-side brief still required for full
              protection. Conservative v1 defaults flagged for
              operator review (sentinel format, per-category
              RBAC granularity, audit-gating, etc.). Brief:
              streams/E1-e-approvals-pii-redaction.md."

Row 7
  brief id:  E1-f-approvals-websocket-live
  owner:     claude
  status:    TODO  (BLOCKED until E0 + E1-a MERGE)
  PR col:    —
  note:      "**Track E slice E1, brief 6/6 (LAST in slice).**
              Reusable WebSocket client wrapper (auto-reconnect,
              exponential backoff + jitter, heartbeat, 3-fail-
              fallback-to-polling) feeds useApprovalsLive hook
              that applies approval.created/decided/undone/
              expired events to TanStack Query cache without full
              refetches. Companion backend brief required for
              structured event payload format; client falls back
              to cache.invalidate-on-event until then. Brief:
              streams/E1-f-approvals-websocket-live.md."

Commit each row insertion with the standard convention, one
commit per row:
  status: new E0-frontend-foundation TODO
  status: new E1-a-approvals-gateway-wireup TODO
  status: new E1-b-approvals-filter-popover TODO
  status: new E1-c-approvals-notifications-popover TODO
  status: new E1-d-approvals-batch-decide TODO
  status: new E1-e-approvals-pii-redaction TODO
  status: new E1-f-approvals-websocket-live TODO

Worker pickup guidance:
  - E0 is a CONTRACT PR. Claim ASAP — its merge unblocks all
    six E1 rows.
  - E1-a is the next critical-path item after E0; it unblocks
    E1-b, E1-d, E1-e, E1-f (E1-c is independent of E1-a).
  - E1-c can claim immediately after E0 MERGES (independent).
  - These briefs require Node 20+, pnpm 9+, and the existing
    web/ scaffold. Workers without a frontend-capable
    environment should skip these rows.

Sibling backend briefs implied (NOT in this batch — authored
separately during Track F):
  - mission-gateway-approvals-json-api — JSON /api/v1/approvals
    + /decide + /undo endpoints (replaces today's HTML /inbox).
    Without this brief, E1-a stays on the mock-fallback path.
  - mission-gateway-approvals-pii-redaction-server-side —
    real PII fence. E1-e is defense-in-depth only without it.
  - mission-gateway-inbox-ws-structured-events — structured
    approval.* events on /inbox/ws. Without this brief, E1-f
    falls back to cache.invalidate-on-any-event.

DO NOT brainstorm replacements for the three sibling backend
briefs — the operator will author them during Track F's
backend-fill-in pass.

Resume your normal tick after the seven row inserts + commits.
Acknowledge by including the new row IDs in your next status
note.
```

## Operator-pending items to review BEFORE workers claim E1-e

E1-e proposes 5 schema decisions defensively with conservative
defaults. Reviewing these BEFORE the worker claims unblocks
them. Each can be answered with one-word picks; reply in chat
and I'll edit the brief in-place.

1. **Default PII patterns.** v1 masks
   email / phone / Luhn-card / IBAN / NZ IRD. Names + addresses
   + free-text are NOT masked. OK for v1? — `[ok / stricter]`
2. **Sentinel format.** `[EMAIL]` vs `***@***.***`. Default
   `[CATEGORY]`. — `[ok / use-asterisks / other]`
3. **Elevated-view request:** log-only or gate-on-admin?
   Default log-only. — `[log-only / admin-gate]`
4. **RBAC permission:** single `unredacted-view` or per-category
   (`unredacted-email`, `unredacted-phone`, etc.)? Default
   single. — `[single / per-category]`
5. **`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` in production:**
   short-circuit unmask (current behavior) or honor RBAC?
   Default short-circuit. — `[short-circuit / honor-rbac]`

E1-f also has 3 questions worth answering, but they're more
mechanical (verifiable from the gateway code):

- `/inbox/ws` auth: header or query-param? Default `?token=`.
- Event-type vocabulary alignment: stick with `approval.created`
  etc. or align with an existing taxonomy?
- LiveIndicator visibility in production: dev-only or
  all-users?

## What this advisory replaces

Nothing. This is the first conductor advisory for Track E. The
two prior advisories (Track B briefs from 2026-05-11) are
unrelated.
