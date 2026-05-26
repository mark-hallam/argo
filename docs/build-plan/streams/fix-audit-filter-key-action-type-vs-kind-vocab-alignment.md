# fix-audit-filter-key-action-type-vs-kind-vocab-alignment

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-filter-key-action-type-vs-kind-vocab-alignment`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/scene-1-and-2-claim-1-and-2.md` F4 (P0 partial — the vocab/key mismatch).

🟠 **P1 — Three different vocabularies for the same concept: the gateway accepts `?action_type=worker_duty.fired`, silently ignores `?kind=worker_duty.fired` (returns *all* rows when given that param), the SPA filter uses `?action_type=`, and the v1.0 demo runbook narrates "`kind =`". A diligent operator transcribing the runbook URL to a curl probe will get the wrong answer back and assume the filter is broken.**

## Root cause

The audit row is internally referred to as `action_type` in the gateway routes (`crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`), `kind` in the audit-event Rust enum (`crates/argo-audit/src/event_kind.rs`), and `verb` in some serializer outputs. The query parameter is `action_type`; `?kind=` is silently ignored (returns all rows instead of erroring) — the bigger violation. The runbook author looked at the enum and wrote `kind =` into Scene 2 narration without checking the wire.

Three places to converge:
1. The gateway query parameter (currently `action_type`).
2. The SPA fetch layer (`web/lib/filters/audit.ts:76-80` — uses `action_type`).
3. The v1.0 runbook narration (uses `kind =`).

Either change the gateway to also accept `?kind=` as an alias (most-compatible — both work), or change the runbook to use `action_type =`. The runbook side is the cheaper fix; the gateway side is the more permissive (operator-friendly) fix.

The dangerous part: `?kind=X` is silently *ignored* (not erroring on unknown query params). That's the bigger bug — it makes the SPA query layer brittle to typos and silently degrades. Erroring on unknown query params would catch this kind of drift at the curl layer.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/scene-1-and-2-claim-1-and-2.md` F4 [P0 partial — the vocab + silent-ignore part] — Gateway silently ignores `?kind=`; SPA uses `?action_type=`; runbook narrates "`kind =`"

## Fix recipe

1. **Server-side compat (recommended)**: in the gateway's audit-feed handler, accept `?kind=` as an alias for `?action_type=` so curl probes following the runbook get the right answer. The audit-event Rust enum is named `Kind` — the wire parameter should mirror the type name.
2. **Server-side strict mode**: any unknown query param on the audit endpoint returns 400 with `{ error: { code: "unknown_query_param", message: "..." } }`. This catches drift at the curl layer and is operator-friendly.
3. **SPA consistency**: update `web/lib/filters/audit.ts:76-80` to use the same name as the gateway (whichever is canonical). Most readable: rename to `kind` everywhere; deprecate `action_type` as alias.
4. **Runbook fix**: the runbook at `docs/skycot/demos/v1-mission-mvp-runbook.md:144-145` should narrate the canonical parameter name (after the SPA + gateway converge).
5. **Contract test**: add `crates/argo-cli/tests/audit_query_param_drift.rs` asserting both `?kind=X` and `?action_type=X` return the same rows, and `?bogus=X` returns 400.

## Acceptance criteria

- [ ] `curl '.../api/v1/audit?kind=worker_duty.fired'` returns the same rows as `?action_type=worker_duty.fired`.
- [ ] `curl '.../api/v1/audit?bogus=X'` returns 400 with `unknown_query_param`.
- [ ] SPA fetch layer and runbook narration agree on a single name.
- [ ] No regression to the audit-feed list endpoint's response shape.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (~20 LOC: accept alias + strict-mode)
- `web/lib/filters/audit.ts` (~5 LOC if renaming)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (1 LOC narration tweak)
- `crates/argo-cli/tests/audit_query_param_drift.rs` (NEW — ~50 LOC contract test)

## Out of scope

- The `verb` field in the serializer (different scope — that's the audit-row payload, not the filter query param).
- The audit-event enum naming itself (`Kind` is the canonical Rust name; leave it).
- Renaming `action_type` in the database schema (additive only — never touch the column name).

## Effort

S — ~100 LOC across handler + SPA + runbook + contract test. ~1.5 hours. P1 because the silent-ignore behavior is a quiet credibility hit on the runbook + a footgun for any operator-curl debugging.
