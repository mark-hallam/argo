# fix-disabled-workflows-policy-self-contradiction

Authored against: origin/main 8286d93b8247ff392fd0f118ea8c0e2c5b980b13 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-disabled-workflows-policy-self-contradiction

## Goal

Fix the self-contradiction in .github/workflows/DISABLED_WORKFLOWS_POLICY.md so
a fresh agent isn't misled about which workflows are active.

## Why now (test-health audit 2026-06-10, F9)

The doc's bottom section ("Active workflows (for reference) — The 5 actively-
enabled workflows are…") describes workflows that have ALL been disabled since
2026-05-21 (every .github/workflows/*.yml is suffixed .disabled; verified). A
fresh agent reading only that section concludes CI is running when it is not.

## Fix

1. Retitle the "Active workflows" section to "Formerly active (pre-2026-05-21)"
   or "Workflows to RE-ENABLE first when GitHub Actions billing returns", and
   state plainly that ZERO workflows run today (admin-merge + local gates only).
2. Cross-check the table classifies every current *.yml.disabled file.
3. Note the re-enable priority order (ci.yml first per the audit) so the
   re-enable path is unambiguous.

## May touch

- .github/workflows/DISABLED_WORKFLOWS_POLICY.md only

## Acceptance / verifier

1. No section claims any workflow is currently active.
2. Every *.yml.disabled in .github/workflows/ is listed/classified.

## Size / owner

Doc-only. Owner: claude (codex-eligible). 🟢 P3 — removes a misleading "CI is
running" signal for fresh agents.
