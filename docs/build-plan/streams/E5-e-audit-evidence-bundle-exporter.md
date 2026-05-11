# Stream E5-e — Audit timeline: evidence bundle exporter (SOC2 / GDPR / HIPAA / Generic)

**Owner**: claude.
**Depends on**: E0 MERGED; E5-a MERGED; E5-c MERGED (single-action export trigger).
**Authored against**: origin/main `6d58f256` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-e-audit-evidence-bundle-exporter`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 5 of 8.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Export generation | Server-side via existing `mission-compliance-soc2-audit-trail` MERGED + `mission-compliance-gdpr-data-export` MERGED + new HIPAA + Generic shapes | Same; v1 ships the UI on top | n/a |
| Format support | SOC2 Type II, GDPR Article 15, HIPAA disclosure log, Generic JSON + PDF | Plus operator-defined custom format templates per tenant | `mission-enterprise-export-custom-templates` |
| Bundle signing | Server signs with workspace-managed key | Per-tenant KMS-rotated signing key | `mission-enterprise-export-kms-signing` (relates to `mission-enterprise-connector-kms-rotation` named in E2-g) |
| Bundle delivery | Immediate download for ≤ 100MB; queued via email for larger | Per-tenant retention policy: signed bundles cached for N days then auto-purged with audit row | `mission-enterprise-export-retention` |
| Export RBAC | Operator-gated for bulk exports involving sensitive PII | Per-format RBAC: SOC2/GDPR may have different operator-tier requirements (e.g. compliance officer vs general operator) | `mission-enterprise-audit-export-rbac` (already named E5-a) |
| Audit-the-audit | Every export is itself audit-logged (uses existing audit infra) | Same | n/a |

v1 is acceptable: export generation runs in the
existing compliance-export infrastructure; this brief
is the UI that wraps it.

## Why

E5-a's `[ ⤓ Export ]` button + E5-c's single-action
`[ ⤓ Export ]` button both open this dialog. This brief
implements the dialog UI: scope (single action /
filtered view / custom range), format (SOC2 / GDPR /
HIPAA / Generic), and the queued delivery / direct
download flow.

For enterprise: this surface IS the compliance
deliverable. Auditors get a signed bundle in one
click; operators get peace of mind in one click.

## Scope

- `EvidenceBundleExporter` dialog opens from:
  - E5-a audit page's `[ ⤓ Export ]` button.
  - E5-c drill-in's `[ ⤓ Export ]` button (single-
    action mode).
  - E5-d replay's `[ ⤓ Export replay artifact ]`
    button (replay-bundle mode).
- Dialog form:
  - **Scope** (radio, 3 options): Current filtered
    view / Specific time range / Sampled audit
    selection.
  - **Time range** (when Scope = Specific): from/to
    date pickers.
  - **Sampled selection** (when Scope = Sampled):
    sample-size input (e.g. 10 random).
  - **Format** (radio, 4 options): SOC2 Type II /
    GDPR Article 15 / HIPAA disclosure log /
    Generic JSON + PDF.
  - **Tenant**: read-only display of current tenant
    (operator may switch in v1.5).
  - **Preview stats**: action count + estimated
    size.
  - **`[ Cancel ]` + `[ ⤓ Export ]`** action row.
- On Export click:
  - Bundle ≤ 50MB → immediate download (browser
    `<a download>`).
  - Bundle 50-100MB → confirm dialog ("This
    export is ~${size}. Continue?") → immediate
    download.
  - Bundle > 100MB → queued mode: "We'll email you
    when it's ready (~${eta})." Toast notification
    on completion.
- Each export emits an audit row.
- Generated bundle includes verifier-script
  reference so auditors can run integrity check
  locally.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: shared via the trigger buttons in
  E5-a/c/d; this brief authors the dialog component.

## Files you may touch

- `web/components/skycot/audit/EvidenceBundleExporter.tsx`
  — NEW. ≤ 400 LOC.
- `web/components/skycot/audit/ExportFormatSelector.tsx`
  — NEW. ≤ 180 LOC.
- `web/components/skycot/audit/ExportScopeSelector.tsx`
  — NEW. ≤ 200 LOC.
- `web/components/skycot/audit/ExportPreviewStats.tsx`
  — NEW. ≤ 150 LOC.
- `web/lib/api/audit-export.ts` — NEW.
  `previewExport(params)` returns
  `{ count, sizeEstimate }`. `generateExport(params)`
  returns `{ kind: "immediate" | "queued", url?,
  eta? }`. Mock fallback. ≤ 250 LOC.
- `web/lib/hooks/use-audit-export.ts` — NEW. ≤ 150
  LOC.
- `web/lib/sample-export.ts` — NEW. Mock preview +
  generation responses per format. ≤ 200 LOC.
- `web/app/(app)/audit/page.tsx` — narrowly scoped:
  wire `[ ⤓ Export ]` to open this dialog. ≤ 430
  LOC after.
- `web/app/(app)/audit/action/[actionId]/page.tsx`
  — narrowly scoped: wire `[ ⤓ Export ]` to open
  this dialog in single-action mode. ≤ 380 LOC
  after.
- `web/components/skycot/audit/EvidenceBundleExporter.test.tsx`
  — NEW. ≥ 10 tests covering scope / format / size
  thresholds / queued flow / audit emission. ≤ 400
  LOC.

## Files you must NOT touch

- All E1-E4 / earlier-E5 files except the narrow
  trigger wiring on E5-a/c.
- `crates/**` — compliance export endpoints already
  shipped via SOC2/GDPR briefs.

## Acceptance criteria

- [ ] Dialog opens from:
      - E5-a audit page `[ ⤓ Export ]` (bulk mode).
      - E5-c drill-in `[ ⤓ Export ]` (single-action
        mode; pre-selects scope = Specific time
        range narrowed to ±1 minute of that
        action's timestamp).
      - E5-d replay `[ ⤓ Export replay artifact ]`
        (replay-bundle mode; pre-selects scope =
        single replay).
- [ ] Dialog header: `"Export evidence bundle"`.
- [ ] Scope radio with 3 options:
      - `Current filtered view`
      - `Specific time range` → opens from/to date
        pickers
      - `Sampled audit selection` → opens sample-
        size number input
- [ ] Format radio with 4 options:
      - `SOC2 Type II` (default)
      - `GDPR Article 15`
      - `HIPAA disclosure log`
      - `Generic JSON + PDF`
- [ ] Tenant: read-only display of current tenant.
- [ ] Preview stats below the form:
      - Action count (computed via `previewExport`).
      - Estimated size (also from preview).
      - Updates on any field change (debounced
        300ms).
- [ ] `[ ⤓ Export ]` button:
      - Enabled when all required fields valid.
      - On click: `generateExport(params)`.
      - Size ≤ 50MB → immediate download via `<a
        download>`; toast confirms.
      - Size 50-100MB → confirm: `"This export is
        ~${size}. Continue?"` then immediate.
      - Size > 100MB → queued mode: dialog
        replaces with `"Your export is being
        prepared. We'll email you when it's
        ready (~${eta})."` + toast notification
        on completion.
- [ ] Each export emits an audit row (mock in v1;
      production via gateway).
- [ ] Format-specific copy in the dialog:
      - SOC2: `"Generates a SOC2 Type II evidence
        bundle (JSON + PDF + hash chain) suitable
        for attestation sampling."`
      - GDPR: `"Generates a per-user GDPR Article 15
        access export (redaction-aware)."`
      - HIPAA: `"Generates a HIPAA disclosure log
        for sampled PHI access."`
      - Generic: `"Generates a documented JSON +
        PDF pair with verifiable hash chain."`
- [ ] Bundle download includes a `verify.js`
      reference + hash-chain instructions in the
      bundle's README.
- [ ] Mock fallback in dev returns fake URLs.
- [ ] 10 unit tests pass.
- [ ] Playwright e2e: open dialog from audit page,
      select format, click Export, see toast
      success.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 10KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-e-audit-evidence-bundle-exporter`
- PR title: `web(audit): evidence bundle exporter dialog (E5-e)`

## Notes for the implementer

- **Dialog primitive.** Radix Dialog from shadcn.
  Modal, dismissible via × + Escape + backdrop
  click.

- **Preview-stats debounce.** Re-fetch every time
  the form changes, debounced 300ms. Server may
  scope-count quickly (< 50ms for ≤ 100K rows).
  v1 mock returns:
  ```ts
  {
    count: estimateRowCount(scope),
    sizeEstimate: count * 4_000,  // ~4KB per row
  }
  ```

- **Format-specific JSON shape.** Server returns
  format-aligned bundles. v1 mock returns a fake
  URL pointing at a `.zip`. Real shapes:
  - SOC2: multi-file zip with `manifest.json`,
    `actions.jsonl`, `policy_log.json`, `signers.txt`,
    `hash_chain.txt`.
  - GDPR: per-user zip with `access_log.json`,
    `data_categories.json`, `redaction_report.json`.
  - HIPAA: zip with `access_log.json`,
    `phi_classification.json`.
  - Generic: `bundle.json` + `bundle.pdf` (paired).

- **Size threshold reasoning.** 50MB is what a
  browser can immediately download cleanly. 100MB
  is the upper limit before background-queue
  delivery feels broken on slow connections.

- **Queued mode UX.** When bundle > 100MB:
  - Dialog body replaced with: `"Your export is
    being prepared. We'll email you when it's
    ready (~${eta})."`
  - `[ Done ]` button closes the dialog.
  - Server emits an email on completion + a push
    notification via the user's primary channel.
  - User can check status under their settings
    (out of scope here; v1.5).

- **Verifier script reference.** Bundle's README:
  ```
  Verify this bundle's integrity:

    1. Install: npm install -g @skycot/verify
    2. Run:     skycot-verify ./bundle.zip

  See https://docs.skycot.com/verify for details.
  ```
  The `verify` CLI is a separate brief (E5-f).

- **Audit-the-audit.** Every export emits a row:
  ```
  POST /api/v1/audit (server-side)
  {
    actor: <viewer>,
    action: "export_generated",
    metadata: { format, scope, sizeEstimate, ... },
    tenant: <tenant>,
  }
  ```

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-export-custom-templates` —
  operator-defined per-tenant format templates.
- `mission-enterprise-export-kms-signing` — per-
  tenant KMS-rotated signing key.
- `mission-enterprise-export-retention` — per-
  tenant retention policy for signed bundles.

Sibling backend briefs (existing):

- `mission-compliance-soc2-audit-trail` (MERGED)
- `mission-compliance-gdpr-data-export` (MERGED)
- NEW: `mission-compliance-hipaa-disclosure-log` —
  HIPAA-format generator.
- NEW: `mission-compliance-generic-export` —
  Generic JSON + PDF generator.
