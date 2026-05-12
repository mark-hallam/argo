# Mission load-bearing wire — Dev-runtime-guardrails doc DEV-ONLY banner

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 9 #271. `docs/dev-runtime-guardrails.md` describes a safety system for long autonomous coding sessions (disk fill, cleanup scripts). A reader landing on the doc could mistake the scripts for production tooling. Add a clear DEV-ONLY banner + cross-references.

## Scope

Edit `docs/dev-runtime-guardrails.md`:

1. **Banner** at the top — bordered callout: "🔧 **DEV / STAGING ONLY** — these scripts are safety nets for autonomous-coding sessions against this repo. They are NOT production tooling. Don't run them on customer-facing SKYCOT deployments. Production cleanup runs through the operational RUNBOOK."
2. **Cross-reference** to `docs/skycot/RUNBOOK.md` for production disk-usage / target-dir cleanup.
3. **Reverse cross-reference** — `docs/skycot/RUNBOOK.md` gets a 1-line "for dev/staging cleanup tooling, see [dev-runtime-guardrails.md](../dev-runtime-guardrails.md)".
4. **Script comments** — each of the 3 cleanup scripts in `scripts/dev-safe-*.sh` gets a `# DEV ONLY` header comment.

Tiny brief — pure docs hygiene.

## Files you may touch

- `docs/dev-runtime-guardrails.md` (≤ 480 LOC after) — banner + cross-ref
- `docs/skycot/RUNBOOK.md` — 1-line reverse cross-ref in the operations section
- `scripts/dev-safe-check.sh`, `scripts/dev-safe-clean.sh`, `scripts/dev-safe-test.sh` — each gets a 5-line `# DEV ONLY` header

## Files you must NOT touch

- The scripts' actual logic (they work as-is)
- Production cleanup paths

## Acceptance criteria

- [ ] DEV-ONLY banner at top of dev-runtime-guardrails.md (bordered callout style)
- [ ] Cross-reference TO RUNBOOK for production cleanup
- [ ] Reverse cross-ref FROM RUNBOOK
- [ ] All 3 dev-safe-*.sh scripts have `# DEV ONLY` header
- [ ] No script logic changed
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-dev-runtime-guardrails-doc-banner`
- PR title: `docs: DEV-ONLY banner on dev-runtime-guardrails (mission-dev-runtime-guardrails-doc-banner)`

## Notes

- 30-min brief. Bias the banner copy toward operator-safety: "running on a production DB will delete customer data" is the right tone.
- The reverse cross-ref makes the production tooling discoverable from the dev doc (which is where Claude agents land first).
