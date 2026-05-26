# fix-skills-marketplace-card-icon-rotation-semantic-name-map

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-marketplace-card-icon-rotation-semantic-name-map`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-4-and-5-claim-4-and-5.md` F13.

🟦 **P3 — `/skills` marketplace card icons rotate from a 5-icon array (`Sparkles / Wrench / Briefcase / TrendingUp / Mail`) applied by index modulo. The pricing-competitor-scrape card gets a Sparkles icon (random); invoice-extract gets Briefcase (also random). Semantically wrong — pricing should be a chart, invoice should be a receipt. Camera reads icons in passing; not load-bearing but reads as "throwaway" to a designer.**

## Root cause

`web/app/(app)/skills/page.tsx:32-38`:
```ts
const ICON_ROTATION = [Sparkles, Wrench, Briefcase, TrendingUp, Mail];
// applied: ICON_ROTATION[index % 5]
```

The index-modulo assignment ignores skill semantics. Result: pricing skill renders Sparkles, invoice renders Briefcase, etc. — random pairings.

The skill name prefix already carries semantic meaning (`pricing.*`, `invoice.*`, `candidate.*`, `lead.*`, `approval.*`, `support.*`). A name-prefix → icon map would do the right thing.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F13 [P3] — Skill card icon rotation uses 5-icon array regardless of skill semantics

## Fix recipe

1. **Build a name-prefix → icon map** in `web/app/(app)/skills/page.tsx` (or a new helper `web/lib/api/skill-icons.ts`):
   ```ts
   const SKILL_ICONS: Record<string, IconComponent> = {
     pricing: TrendingUp,          // chart-up
     invoice: Receipt,
     candidate: Users,
     lead: Target,
     approval: ShieldCheck,
     support: LifeBuoy,
     market: BarChart3,
     deal: HandshakeIcon,
     // fallback
     default: Sparkles,
   };

   function iconForSkill(name: string): IconComponent {
     const prefix = name.split('.')[0];
     return SKILL_ICONS[prefix] ?? SKILL_ICONS.default;
   }
   ```

2. **Replace `ICON_ROTATION[index % 5]`** with `iconForSkill(skill.name)` at the card render site.

3. **Verify lucide-react / your icon library has the suggested icons**; substitute as needed (Receipt, Target, ShieldCheck, LifeBuoy, BarChart3 are all in lucide).

4. **Vitest pin**: each known seed skill prefix maps to its expected icon (snapshot test).

5. Same name-prefix pattern could carry to the workflow card icons + audit-row icons if desired (file a follow-up brief if so).

## Acceptance criteria

- [ ] Pricing skills render a chart/trend icon. Invoice skills render a receipt icon. Lead skills render a target icon. etc.
- [ ] No `ICON_ROTATION[index % N]` pattern remains in `skills/page.tsx`.
- [ ] Vitest snapshot pin passes.
- [ ] Camera-readable: each card's icon is recognisable as a hint to the skill's domain.

## May-touch

- `web/app/(app)/skills/page.tsx` (icon assignment)
- `web/lib/api/skill-icons.ts` (NEW — name-to-icon map)
- `web/components/skycot/skills/SkillCard.tsx` (if extracted)
- Vitest test

## Out of scope

- Workflow card icons (file separately if desired)
- Audit-row icons (separate)
- Icon design itself (use lucide-react primitives — no custom SVGs)

## Effort

XS — ~30-50 LOC. ~30 min.
