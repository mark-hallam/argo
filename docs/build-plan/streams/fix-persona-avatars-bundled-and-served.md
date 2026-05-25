# fix-persona-avatars-bundled-and-served

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-persona-avatars-bundled-and-served`.
**Source**: e2e-ux-audit (scene-1-and-2-claim-1-and-2) finding F7 — `docs/skycot/ux-audits/2026-05-26-0820/scene-1-and-2-claim-1-and-2.md`.

🟡 **P1 — Workers have no photo. Trading-card deck shows text initials in colored circles. Claim 1 "teammate, not tool" reads as "config rows with avatars" rather than colleagues.**

## Root cause

Gateway returns `avatar_url: null` for every worker:
```
curl /api/v1/team → "workers":[{"name":"Maya","avatar_url":null,...}]
```

`web/public/personas/` and `web/public/avatars/` directories **do not exist**. `SamplePersona` type doesn't carry an `avatarUrl` field. The `<Avatar>` component falls back to text initials (e.g. "M" for Maya) in a colored circle.

Per Claim 1 spec ("name, photo, personality traits"), the **photo dimension is absent**. The trading-card deck (locked C2-holographic design per memory `project_workers_page_design_locked`) renders 5 initial-circles where it should render 5 distinct headshots. The camera will catch initials-only avatars on every wide shot of `/team` and `/workers/<id>` hero.

This is dressing-side (the personas are fictional, the photos are synthetic), but the gap is camera-visible on every Scene 1 + Scene 2 + Scene 6 shot. For an investor demo where "teammate not tool" is the load-bearing register, initials-only avatars erode the metaphor.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F7 [P1] — Workers have no photo; gateway returns avatar_url=null; no persona PNGs bundled

## Fix recipe

1. **Generate persona headshots** for the 6 canonical demo personas (Mariah, Peta, Priya, Jamal, Nori, Maya) — plus Patrick once shipped. Use a consistent style:
   - Square, 512×512 WebP source
   - Friendly, professional headshot register (not corporate-stiff, not cartoonish)
   - Diverse representation
   - Generate via a stable diffusion / Midjourney / commercial AI portrait tool — operator should approve final selections (this is a brand surface; ask the operator if any synthetic-image policy or specific generator preference applies)
   - File naming: `<persona-id-slug>.webp` (e.g. `mariah.webp`, `peta.webp`)

2. **Store at `web/public/personas/`** — create directory if missing. Commit the binary WebP files (small files, fine in git for v1; consider CDN later).

3. **Add `avatarUrl` to `SamplePersona`** at `web/lib/sample-personas.ts`:
   ```ts
   interface SamplePersona {
     id: string;
     name: string;
     avatarUrl: string;  // NEW — e.g. "/personas/mariah.webp"
     // ... existing fields
   }
   ```
   Populate for all 6 personas + Patrick.

4. **Gateway emits non-null `avatar_url`** when the worker's persona has a bundled headshot:
   - Touch `crates/argo-cli/src/handlers/gateway/routes_team_list.rs` (team list emitter)
   - Touch `crates/argo-cli/src/handlers/gateway/routes_workers_overview*.rs` (worker overview emitter)
   - Lookup table: `worker.persona_id → avatar_url` from a small registry in the vertical pack manifest, OR a hardcoded map keyed by persona name (acceptable for v1)
   - For workers without a registered persona (new hires named anything else), keep `avatar_url: null` and let the SPA fall back to initials

5. **SPA renders the avatar when present** — verify `<Avatar src={worker.avatarUrl}>` falls back to initials when src is null/empty. This is already the contract for `<Avatar>`; just ensure no caller hardcodes initials when src is truthy.

6. **Touch sites that render avatars**:
   - `web/components/skycot/TeammateCard.tsx` (deck card)
   - `web/components/skycot/WorkerHero.tsx` (worker detail hero)
   - `web/components/skycot/ActivityRow.tsx` (audit feed row)
   - `web/components/skycot/approvals/ApprovalRow.tsx` (approvals row)
   - `web/components/skycot/cohorts/TileMeta.tsx` (cohort tile)
   - Verify each passes `worker.avatarUrl` through to `<Avatar>`.

7. **Storybook update**: refresh persona storybook variants so designers + reviewers can spot-check the headshots.

## Acceptance criteria

- [ ] `web/public/personas/` contains WebP headshots for Mariah, Peta, Priya, Jamal, Nori, Maya (+ Patrick if shipped).
- [ ] `SAMPLE_PERSONAS` in `web/lib/sample-personas.ts` carries an `avatarUrl` field on each canonical persona.
- [ ] `GET /api/v1/team` returns non-null `avatar_url` for workers whose persona has a bundled headshot.
- [ ] `/team` trading-card deck renders headshots (not initial circles) for all 5 seeded workers.
- [ ] `/workers/<id>` hero renders the same headshot.
- [ ] Audit feed rows + approvals rows + cohort tiles render the headshot when available.
- [ ] Workers without a registered persona fall back to initials gracefully (no broken-image icon).
- [ ] Image files load with proper cache headers (long-cache; immutable filename).
- [ ] Storybook variants updated.

## May-touch

- `web/public/personas/*.webp` (NEW — binary files; operator-approved set)
- `web/lib/sample-personas.ts` (add avatarUrl field)
- `crates/argo-cli/src/handlers/gateway/routes_team_list.rs` (emit avatar_url)
- `crates/argo-cli/src/handlers/gateway/routes_workers_overview*.rs` (emit avatar_url)
- `crates/argo-vertical-pack/src/manifest.rs` (avatar registry, if going the manifest route)
- `web/components/skycot/TeammateCard.tsx`, `WorkerHero.tsx`, `ActivityRow.tsx`, etc. (verify avatar src wiring)
- `web/stories/` (persona variants)

## Out of scope

- Per-tenant white-label avatars / customer-uploaded headshots (Phase 2)
- AI-generated avatars at hire time (Genesis Phase 2)
- Avatar cropping / editing UI in settings (Phase 2)

## Effort

~50-100 LOC + 6-7 small WebP files. ~2 hours code + asset-generation time. Operator approval needed on final headshot set before commit.
