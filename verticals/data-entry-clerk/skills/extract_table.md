---
name: extract_table
description: Given a source document and a target schema, emit one row per table line that matches the schema, with per-field confidence and source spans. Leaves any unreadable field blank.
inputs:
  - document_id
  - target_schema_id
  - pages
outputs:
  - rows
  - _page_coverage
  - _escalations
---

# Skill: Extract Table

Given a document containing a table (line-item invoice, expense
export, contact list, inventory sheet) and the id of an operator-
defined target schema, produce one row per table line.

1. **Resolve the schema.** Look up `target_schema_id` in
   `target_schemas` memory or `inventory.json`. If unknown, refuse
   and escalate — never guess the column set.
2. **Find the table header.** Anchor extraction on the header row.
   Reject every body row whose column count does not match the
   header. Surface mismatch counts in `_escalations`.
3. **Per-cell read.** For each cell, record value + confidence +
   source span. A cell whose OCR is below the `0.70` floor is left
   blank in the value and added to the row's `_escalations` list.
4. **Type coerce per column.** Cast strings to ints / decimals /
   dates / enums as the schema declares. A coercion failure (e.g.
   `"$1,2345.00"`, `"2026-13-32"`) leaves the field blank and
   escalates — never silently round, trim, or reformat ambiguous
   input.
5. **Stitch wrapped tables across pages** by re-anchoring on the
   header on each subsequent page. If the header is absent on a
   continuation page, escalate the page.
6. **Emit one row per body row** with `_confidence`, `_source_spans`,
   and `_escalations`. Add the affected page numbers to
   `_page_coverage`.

NEVER fabricate column values to make a row "look complete". Blank +
escalation is the safe state.

Output shape:

```json
{
  "rows": [
    {
      "row_id": "r1",
      "values": {"sku": "SK-42", "qty": 10, "unit_price": null},
      "_confidence": {"sku": 0.95, "qty": 1.00, "unit_price": 0.0},
      "_source_spans": {"sku": {"page": 2, "bbox": [120, 480, 220, 500]}},
      "_escalations": [
        {"field": "unit_price", "reason": "currency symbol illegible"}
      ]
    }
  ],
  "_page_coverage": [1, 2, 3],
  "_escalations": []
}
```
