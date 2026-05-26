# fix-onboarding-first-hire-creates-orphan-daily-check-in-duty

**Owner**: codex.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-creates-orphan-daily-check-in-duty`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F7 [P1].

🟠 **P1 — First-hire creates a duty with `entry_action: "daily_check_in"` and cron `"0 9 * * 1-5"`. No worker, skill, or workflow registers that action name anywhere in the codebase. The "Run now" beat in Scene 2 fires a duty that has no handler — camera reads "duty fired but no result."**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-273` hardcodes:
```rust
DutyDraft {
    entry_action: "daily_check_in".to_string(),
    cron: "0 9 * * 1-5".to_string(),
    ...
}
```
A grep `rg 'daily_check_in' crates/` returns only this one line — no registered handler, no skill action, no workflow that recognises the string. Duty scheduler will tick at 09:00 Mon-Fri, attempt to fire `daily_check_in`, and either no-op or error silently. Either way the audit feed shows a duty fire with no actual agent turn.

## Code evidence

```bash
$ rg 'daily_check_in' crates/
crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:268:  entry_action: "daily_check_in".to_string(),
(no other hits — no handler, no skill, no workflow)
```

## Findings this brief closes

- onboarding-hire-an-ai-teammate F7 [P1] — Hardcoded duty `daily_check_in` is a string with no handler anywhere

## Fix recipe

Pick ONE of (a) or (b).

**(a) Auto-pick a real entry_action from the installed vertical's skill set:**
1. After `install_vertical` lands skills, look up the vertical's manifest for a "default daily skill" hint.
2. Use that action name as the duty's `entry_action`.
3. If the vertical doesn't declare a default-daily, fall back to the first daily-tagged skill from the install set.
4. If none, skip creating an auto-duty (preferred fallback — see (b)).

**(b) Stop auto-creating a duty in `first-hire`; let the operator add the first duty manually:**
1. Remove the `DutyDraft` block from `routes_onboarding.rs:267-273`.
2. Add a one-line onboarding-success toast: "Add their first responsibility from their profile."
3. Step 5's redirect to `/workers/{id}` already lands on Overview, which already shows the Responsibilities card — the operator can click "+ Add responsibility" immediately.

Strongly prefer (b) — it eliminates the orphaned-action surface entirely and aligns with the "manage your teammate" loop that the worker-detail page is designed around. Option (a) is fragile because new verticals can land without remembering to register a default-daily skill.

## Acceptance criteria

- [ ] `daily_check_in` is no longer emitted by `routes_onboarding.rs`.
- [ ] First-hire response no longer includes a `duty_id` (or includes `duty_id: null` with a clear semantic).
- [ ] Audit feed no longer contains duty-fire events with no underlying agent turn.
- [ ] Worker overview Responsibilities card on a freshly hired worker shows the empty state with the "+ Add responsibility" CTA.
- [ ] Existing test `crates/argo-cli/tests/onboarding_first_hire_*.rs` updated (or new test added) asserting no orphan duty is created.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-273` (remove or rewrite)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (if duty helper is shared)
- `crates/argo-cli/tests/onboarding_first_hire_*.rs` (assertion update)

## Out of scope

- Vertical-manifest schema for "default daily skill" hint (deferred — option (a) is rejected here)
- Worker-detail Responsibilities `+ Add responsibility` UI (already exists)
- Duty scheduler error reporting for unknown actions (general improvement, separate brief)

## Effort

~30-50 LOC + test update. ~1 hour.
