---
name: reconcile_against_source
description: Given an internal record and a source document (PDF / scan / email), diff field-by-field and emit a reconciliation report with disagreement count and a suggested resolution per field. Output Markdown plus JSON. Approval-gated for any record mutation.
inputs:
  - record
  - source_document_id
  - field_map
outputs:
  - report_markdown
  - report_json
  - _escalations
---

# Skill: Reconcile Against Source

Compare an internal record against the source document it claims to
represent (an invoice scan, a signed contract PDF, an email body) and
emit a field-by-field reconciliation report. The skill is read-only:
it NEVER mutates the record. Any resolution that would change a
stored value routes through the operator-approval gate.

1. **Resolve the field map.** `field_map` declares which internal
   field corresponds to which source-document element (header cell,
   line item, signature block, body span). If absent, refuse and
   escalate — reconciliation without a map silently misaligns
   columns and produces dangerous false-confidence diffs.
2. **Extract from the source.** Pull each mapped value with per-field
   confidence and a source span (page, bbox). Source confidence below
   the `0.70` floor counts as "unreadable" and the field is flagged
   `source_unreadable` rather than diffed.
3. **Diff per field.** For each mapped field, compute one of:
   - `match` — values equal after type-aware normalization (currency
     rounded to declared precision, dates parsed to YYYY-MM-DD,
     addresses normalized via `normalize_address`).
   - `mismatch` — values differ.
   - `source_unreadable` — source confidence below `0.70`.
   - `record_missing` — internal record has no value.
   - `source_missing` — source has no value.
4. **Suggest a resolution** per mismatch:
   - `accept_source` — source confidence ≥ 0.95 AND record confidence
     unknown or < source confidence.
   - `accept_record` — record has a higher-confidence audit trail
     (e.g. operator-confirmed) or the source span is partly occluded.
   - `operator_review` — neither side dominates, OR the field is in
     the vertical's `human_approval_required` set.
   Suggestions are hints. The skill NEVER applies a resolution; any
   record mutation downstream goes through the approval gate.
5. **Aggregate**: count mismatches, list which fields disagree, and
   set `report_json.ok = true` only if every field is `match`. A
   single mismatch flips `ok` to false and pushes the report into
   the operator queue.
6. **Render Markdown** with a side-by-side table — operators eyeball
   diffs faster than they read prose. Columns: `Field | Record |
   Source | Status | Suggested Resolution`. Source spans (page
   numbers) go in the footer so the operator can jump to the page.

NEVER coerce a mismatch into a match by widening tolerances. NEVER
auto-apply a suggested resolution. NEVER omit a `source_unreadable`
field — the operator must see gaps, not just disagreements.

Output shape (`report_json`):

```json
{
  "ok": false, "mismatch_count": 2,
  "fields": [
    {"field": "invoice_total", "record_value": 1100.00, "source_value": 1150.00,
     "status": "mismatch", "source_confidence": 0.98,
     "suggested_resolution": "accept_source",
     "source_span": {"page": 1, "bbox": [400, 720, 480, 740]}},
    {"field": "due_date", "record_value": "2026-05-31", "source_value": null,
     "status": "source_unreadable", "source_confidence": 0.42,
     "suggested_resolution": "operator_review",
     "source_span": {"page": 1, "bbox": [120, 200, 240, 220]}}
  ],
  "_escalations": [
    {"field": "invoice_total", "reason": "mismatch"},
    {"field": "due_date", "reason": "source_unreadable"}
  ]
}
```

Markdown shape — side-by-side table, source spans in footer:

```markdown
# Reconciliation Report — record `<record_id>` vs document `<doc_id>`

| Field         | Record     | Source       | Status            | Suggested Resolution |
|---------------|------------|--------------|-------------------|----------------------|
| invoice_total | 1100.00    | 1150.00      | mismatch          | accept_source        |
| due_date      | 2026-05-31 | _unreadable_ | source_unreadable | operator_review      |

Spans: invoice_total p.1 [400,720,480,740]; due_date p.1 [120,200,240,220].
```
