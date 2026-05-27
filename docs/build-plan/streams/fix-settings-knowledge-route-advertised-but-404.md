# fix-settings-knowledge-route-advertised-but-404

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-settings-knowledge-route-advertised-but-404`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — Settings tabs registry advertises a "Knowledge" tab pointing at `/settings/knowledge`, but the route doesn't exist (404). Clickable from every settings page.**

## Root cause

`web/components/skycot/settings/SettingsTabs.tsx:57-61` defines a "Knowledge" tab pointing at `/settings/knowledge`. The route does not exist:

```
$ curl -s -o /dev/null -w "%{http_code}\n" http://localhost:61264/settings/knowledge
404
```

— `whole-site-mvp-claims-coverage.md` F19

The tab was likely scaffolded for a future feature (knowledge-base / SOP-upload surface relevant to Genesis / `feedback_genesis_is_ai_teammate_onboarding_studio`) but the page was never landed. Tab is clickable from every settings page and any investor poking at it sees a 404.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F19 [P2] — `/settings/knowledge` returns 404 (route advertised in tab list)

## Fix recipe

Two operator-decisional options:

### Option A — Ship a placeholder Knowledge page

1. Create `web/app/(app)/settings/knowledge/page.tsx` that renders:
   - A SettingsShell-wrapped page with title "Knowledge" + breadcrumb
   - Empty state explaining the surface: "Upload SOPs, paste wiki URLs, or link Slack threads so your AI teammates learn how your team operates. This surface ships in v1.1 — use Genesis for now."
   - A `<Link href="/genesis">Open Genesis →</Link>` CTA
2. Add `metadata` export with `title: "Knowledge · SKYCOT"`.
3. Per `feedback_no_theater_in_demo`, no fake upload boxes — clearly mark as "coming next".

### Option B — Remove the tab from the registry

1. Delete the Knowledge entry from `SettingsTabs.tsx:57-61`.
2. Remove any imports or test fixtures referencing it.
3. Note the deferred feature in `docs/build-plan/MISSION_BACKLOG.md` for v1.1.

Option A preserves the visual cue that Knowledge is in scope; Option B is the lower-risk MVP fix.

## Acceptance criteria

- [ ] `/settings/knowledge` no longer returns 404 (Option A) OR the tab is removed from the registry (Option B).
- [ ] No clickable links lead to a 404.
- [ ] Component test confirms the tab list or the page renders correctly.
- [ ] PR body cites finding F19.

## May-touch

- `web/components/skycot/settings/SettingsTabs.tsx`
- `web/app/(app)/settings/knowledge/page.tsx` (NEW, Option A only)
- `web/components/skycot/settings/SettingsTabs.test.tsx`

## Out of scope

- Building the actual knowledge-base feature (post-v1)
- Genesis multi-modal upload (`feedback_genesis_is_ai_teammate_onboarding_studio` — separate stream)
- Settings shell double-h1 (covered by `fix-settings-shell-double-h1-and-danger-zone-redirect`)

## Effort

XS — Option B: ~10 LOC. ~20 minutes.
S — Option A: ~80-120 LOC + test. ~2 hours.
