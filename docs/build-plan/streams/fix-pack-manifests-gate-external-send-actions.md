# fix-pack-manifests-gate-external-send-actions

**Owner**: claude.
**Authored against**: origin/main `d0ee42aee` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pack-manifests-gate-external-send-actions`.

## Why (evidence-grounded — deploy-readiness run 2026-06-08, finding V10)
Common archetypes FAIL at hire because their pack manifests ship external send/publish actions as **autonomous, no-approval**. The Genesis safety judge then correctly BLOCKS the hire — its exact words: *"The charter says 'never sends until I approve' but the vocabulary grants the capability regardless; the vocabulary must be restricted to draft-only actions."* Determinism probe: marketing-intern role × 3 → 3/3 blocked, identical reason. This is the truth-boundary working correctly; the real bug is the **manifest action defaults**. Impact: real users minting common roles (marketing intern, etc.) can't complete a hire. **Conversion/usefulness blocker, NOT a security hole.** Low blast radius (pack TOML data).

## Grep-before-write (confirm the exact offenders at claim time)
- `rg -n 'requires_approval' verticals/*/vertical.toml` — find every action block.
- `rg -ln 'post_campaign|send_followup|send_email|publish|post_to|send_message|send_followup_email' verticals/*/vertical.toml` — the external send/publish actions.
- For EACH such action whose `[actions.<name>]` block has `requires_approval = false` (or omits it) AND which performs an EXTERNAL mutation (sends/publishes to a real channel), set `requires_approval = true`. Do NOT touch internal/draft/read actions (draft_*, summarize_*, research_*, log_to_* internal) — those should stay auto.
- Pattern reference (already correct): `verticals/sales-follow-up/vertical.toml` `[actions.send_followup_email]` + its eval seed `sfu_send_email_requires_approval` (external_mutation pending+approval). Mirror that.

## Files you may touch
- `verticals/<archetype>/vertical.toml` for each affected pack (at minimum `marketing-intern`; audit ALL packs and fix every external-send/publish action that is auto-approved). TOML data only.
- If a pack has an `eval/` seed corpus, add/adjust one seed per newly-gated action asserting it returns `requires_approval` + `STOP: APPROVAL_REQUIRED` and does NOT externally mutate (mirror `sfu_send_email_requires_approval`).

## Acceptance (truth-bounded — current-turn evidence)
1. Enumerate (in the PR body) every action you changed: `pack / action / before→after`.
2. Re-run the hire that was blocked: `POST /api/v1/genesis {"role_description":"<marketing intern role>"}` → the safety_verdict must now be `allow` (or the affected archetype hires cleanly via `/api/v1/onboarding/first-hire`). Paste the gate verdict. (Gateway on :7400; or assert via the genesis gate unit path if no live gateway.)
3. No internal/draft/read action was newly gated (don't over-correct — that would break auto-draft).
4. `cargo test -p argo-vertical` (manifest load/validate) green; any new eval seeds parse.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (vertical-packs safety-defaults row) + `python3 scripts/render-feature-matrix.py`. Note in the PR that this clears deploy-readiness finding V10.

## Effort
S–M (data fix across N packs + eval seeds). Pure-mechanical once the offender list is greped.
