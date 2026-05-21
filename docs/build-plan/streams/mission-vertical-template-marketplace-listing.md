# mission-vertical-template-marketplace-listing

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-template-marketplace-listing`.

## Summary

[CLAUDE.md](CLAUDE.md) project invariant: "Vertical agents must be implemented through templates, not hardcoded runtime forks. Vertical-specific behavior belongs in: manifests, workflow definitions, prompt packs, memory schemas, tool permissions, escalation policies, evaluation cases, UI/gateway metadata."

The substrate exists (vertical-templates are loaded at provision time), but there's no end-user surface that browses available verticals. The `/onboarding` flow's "vertical" step (#1955) shows a hardcoded set. This brief wires a real marketplace listing: a `/verticals` (or `/marketplace/verticals`) page that lists vertical-templates from the registry, with detail pages showing manifest + included skills + sample workflows.

## Acceptance

- [ ] `/verticals` (or `/marketplace/verticals`) page lists every vertical-template registered in the canonical registry (read `crates/argo-vertical-templates/` or wherever the registry lives — `rg 'pub fn .*vertical|VerticalTemplate' crates/` first).
- [ ] Each card shows: vertical name, headline, supported worker roles, sample-workflow count.
- [ ] `/verticals/[verticalId]` detail page shows: manifest contents (workflows + prompt packs + memory schemas), expected setup time, illustrative worker examples.
- [ ] Listing is sourced from real registry data, not hardcoded. Verify the page updates if a new vertical lands by adding a new template entry.
- [ ] Locked register: brand-loud marketplace (matches `/skills` A1 register per operator memory `project_skills_page_design_locked.md`). Reuse the hero-search pattern.
- [ ] Onboarding "vertical" step (already shipped in #1955) is NOT broken — its hardcoded list either continues to work or is migrated to the new endpoint. Pick whichever is fewer LOC.
- [ ] At least one integration test on the new gateway endpoint.

## May touch

- `web/app/(app)/verticals/page.tsx` + `web/app/(app)/verticals/[verticalId]/page.tsx` (new)
- `web/components/skycot/verticals/` (new component module, mirroring `cohorts/` and `skills/` shape)
- `web/lib/api/verticals.ts` (gateway client)
- `crates/argo-gateway/src/routes_verticals.rs` (or equivalent) — additive endpoint(s)
- New integration test under the gateway crate's `tests/`

## Must NOT touch

- The vertical-template registry internals — read-only surface.
- The runtime vertical-routing logic.
- `/onboarding` form submission contract (only the "vertical" step's data source may change).
- Other marketplace surfaces (`/skills` is locked).

## Notes

- Auditor signal: claim 3 ("Platform builds workers") substrate is complete but the **discoverability surface is missing** — buyers can't browse what they can install.
- Pre-flight: `rg -l 'VerticalTemplate' crates/` to find the canonical types.
- Sizing: 500–800 LOC across backend additive endpoint + UI listing + detail page.
- A natural mission-claim-3 procurement surface; pairs well with the skill discovery brief.
