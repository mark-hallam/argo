# fix-settings-read-only-panels-no-onchange-no-state-wireup

**Owner**: codex.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-read-only-panels-no-onchange-no-state-wireup`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/whole-site-mvp-claims-coverage.md` F13 [P2].

🟡 **P2 — 3 of 4 user-editable Settings sub-pages render hardcoded React strings with zero `useState`, zero `onClick`, zero `onChange`: `/settings/workspace`, `/settings/providers`, `/settings/notifications`. Values like "SKYCOT Demo", "Marketing operations", "Australia/Melbourne", "Anthropic" are baked into JSX. The pages are presented as if these are configurable settings; the operator can't actually change anything.**

## Root cause

Per the audit walk: `/settings/workspace/page.tsx`, `/settings/providers/page.tsx`, `/settings/notifications/page.tsx` are "looks-like-a-form, isn't-a-form" surfaces. Each renders the locked-design Settings layout (D top-tabs per memory `project_settings_page_design_locked`) with input fields and dropdowns — but no event handlers, no controlled state, no submit button wired to any mutation.

The 4 settings sub-pages that ARE functional (verified in the audit):
- `/settings/api-keys` — real CRUD via `useApiKeys` / `useCreateApiKey` / `useRevokeApiKey`
- `/settings/data-export`
- `/settings/members`
- `/settings/security`

Per function-vs-dressing rule: settings is dressing for the POC investor demo (camera doesn't drill in), so this is not a v1.0 demo blocker. It IS a customer-onboarding gap that hits the moment a real customer signs up and tries to change their workspace name or notification preferences.

Three sub-pages × ~3-5 fields each ≈ 9-15 wired endpoints needed. Most settings already have gateway equivalents (the demo SQL has `workspace.name`, `workspace.timezone`, etc.) — the SPA-side wire-up is the missing layer.

## Findings this brief closes

- whole-site-mvp-claims-coverage F13 [P2] — Settings panels are read-only-with-hardcoded-values; no onClick/onChange on 3 of 4 user-editable surfaces

## Fix recipe

This is a multi-page effort — recommend splitting into 3 sub-briefs (one per page) if a single PR is too large. The pattern is shared across all three:

1. **Per page**: replace hardcoded strings with `useState` controlled inputs. Initial state from a `useQuery` against the appropriate gateway endpoint.
2. **Wire submit handlers**: each editable field has an inline save (debounced 800ms) OR a single "Save" button at the bottom. Locked design favors inline-save for the workspace + providers panels; notifications may want a single Save (more fields, fewer mutations).
3. **Gateway endpoints**:
   - `/settings/workspace` → `PATCH /api/v1/workspace { name, description, timezone }` — verify endpoint exists; if not, ship it (small Rust handler).
   - `/settings/providers` → `PATCH /api/v1/providers/{providerId} { ... }` or `POST /api/v1/providers { ... }` for new providers.
   - `/settings/notifications` → `PATCH /api/v1/notifications/preferences { ... }`.
4. **Optimistic updates** for inline-save UX: update local state immediately, revert on error toast.
5. **Vitest** per page — assert input is controlled, change events fire, submit calls the mutation.
6. **Auth-bypass / demo-mode**: respect the same flag the other settings pages use — in demo mode, simulate the mutation locally without round-tripping to gateway.

## Acceptance criteria

- [ ] `/settings/workspace` — Name, Description, Timezone fields are controlled inputs; editing them fires a mutation (or simulates in demo mode).
- [ ] `/settings/providers` — Provider selection / API key fields editable; save fires `PATCH`.
- [ ] `/settings/notifications` — Channel preferences editable; save fires `PATCH`.
- [ ] Optimistic update + revert-on-error toast.
- [ ] No regression on the 4 sub-pages that are already functional (`api-keys`, `data-export`, `members`, `security`).
- [ ] Vitest pin per page.

## May-touch

- `web/app/(app)/settings/workspace/page.tsx`
- `web/app/(app)/settings/providers/page.tsx`
- `web/app/(app)/settings/notifications/page.tsx`
- `web/lib/api/settings.ts` (new mutations + queries)
- Gateway-side: any missing `PATCH /api/v1/workspace` / `/providers/{id}` / `/notifications/preferences` handlers
- Vitest fixtures per page

## Out of scope

- Settings sub-page metadata (covered by `fix-settings-sub-pages-add-missing-metadata`)
- Settings shell H1 hierarchy (covered by `fix-settings-shell-double-h1-and-danger-zone-redirect`)
- Theme picker scoping (covered by `fix-spa-theme-picker-only-on-approvals-not-settings`)
- The 4 sub-pages that ARE wired (`api-keys`, `data-export`, `members`, `security`)

## Effort

L — ~300-500 LOC across SPA + possibly gateway endpoints + tests. ~6-10 hours. Recommended to split into 3 sub-briefs (workspace + providers + notifications) for parallel claims. P2 because investor demo never drills into these; customer-trial blocker.
