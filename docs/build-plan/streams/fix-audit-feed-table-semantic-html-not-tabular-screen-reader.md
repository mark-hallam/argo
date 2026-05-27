# fix-audit-feed-table-semantic-html-not-tabular-screen-reader

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-feed-table-semantic-html-not-tabular-screen-reader`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-6-claim-6-trust-and-approvals.md` F10 [P3].

🟢 **P3 — `/audit` Etherscan-style table renders rows as `<div role="list">` not a real `<table>`. Screen readers announce "list, 31 items" rather than "table, 31 rows, columns: time, worker, verb, id, status." Etherscan itself uses a real `<table>`. The locked design memory (`project_audit_page_design_locked`) doesn't specify markup — only visual.**

## Root cause

`web/components/skycot/audit/AuditDayGroup.tsx` renders the day-grouped feed using:

```
<section>
  <h2>{day}</h2>
  <div role="list">
    <div data-status="..." ...>...</div>
    ...
  </div>
</section>
```

This is visually accurate to the locked B3-Etherscan-style design (the camera reads correctly), but semantically the structure is a list, not a table. Screen readers can't announce column headers or navigate by column. Etherscan's actual implementation uses a `<table>` with `<thead>` / `<tbody>` / `<tr>` / `<td>` — keyboard users can press T to jump between tables and navigate columns with the table-rotor.

This is a polish-grade accessibility nit. The camera doesn't read DOM semantics, so the v1.0 demo MP4 is unaffected. But customers using screen readers will notice.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F10 [P3] — `/audit` Etherscan-style table renders rows as `<div role="list">` not `<table>`; screen-reader announces ambiguous structure

## Fix recipe

1. **Refactor `AuditDayGroup.tsx`** to use real table markup:
   ```tsx
   <section aria-labelledby={dayHeadingId}>
     <h2 id={dayHeadingId}>{day}</h2>
     <table>
       <caption className="sr-only">Audit rows for {day}</caption>
       <thead>
         <tr>
           <th scope="col">Time</th>
           <th scope="col">Worker</th>
           <th scope="col">Verb</th>
           <th scope="col">Id</th>
           <th scope="col">Status</th>
         </tr>
       </thead>
       <tbody>
         {rows.map(row => <AuditRow key={row.id} row={row} />)}
       </tbody>
     </table>
   </section>
   ```
2. **Update `AuditRow.tsx`** to render `<tr>` + `<td>` (or `<th scope="row">` for the leading cell) instead of `<div>` siblings.
3. **Preserve the visual design.** The locked B3-Etherscan style uses Inter font + hover state + status-dot colour-coding. Tailwind table classes preserve this; use `display: table` if needed.
4. **`<thead>` may be visually hidden** if the locked design doesn't show column headers (Etherscan does, the locked design likely does too — verify). If hidden, use `sr-only` on `<thead>` so screen readers still announce columns.
5. **Test with a real screen reader** (VoiceOver on macOS): assert that pressing CTRL+ALT+T jumps to the table and the column headers announce.
6. **Vitest** asserting tag tree shape (table/thead/tbody/tr/th/td).

## Acceptance criteria

- [ ] `/audit` renders rows inside a `<table>` with `<thead>` + `<tbody>` + `<tr>` + `<td>`.
- [ ] Screen reader announces "table, N rows, columns: time, worker, verb, id, status".
- [ ] Visual design unchanged (Etherscan-style preserved).
- [ ] Vitest assertion on the new tag structure.
- [ ] No regression on hover/click behaviour (clicking a row still navigates to detail).

## May-touch

- `web/components/skycot/audit/AuditDayGroup.tsx` (wrap in table)
- `web/components/skycot/audit/AuditRow.tsx` (render as `<tr>`)
- `web/components/skycot/audit/__tests__/AuditDayGroup.test.tsx` (extend)
- Possible Tailwind class adjustments to preserve visual fidelity

## Out of scope

- Audit feed self-traffic filter (covered by `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`)
- Audit-detail rendering hygiene (covered by `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`)
- The `/audit/action/<id>` detail page semantics (separate route)

## Effort

S-M — ~60-120 LOC. ~1-2 hours. P3 because no demo scene blocks on this; pure accessibility polish for customer trial.
