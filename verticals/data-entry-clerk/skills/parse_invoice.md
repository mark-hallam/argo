---
name: parse_invoice
description: Given an invoice scan or PDF, output vendor, invoice number, dates, totals, and line items with per-field confidence and source spans. Leaves any ambiguous field blank.
inputs:
  - document_id
  - pages
  - vendor_hint
outputs:
  - invoice
  - line_items
  - _confidence
  - _escalations
---

# Skill: Parse Invoice

Read an invoice document and emit a typed record. Header fields live
on page 1; line items often span pages; totals usually land last.

1. **Identify the vendor.** Match against `vendor_aliases` memory.
   If a match exists with confidence ≥ 0.90, prefer the canonical
   alias. If no match, record the raw vendor string and escalate.
   Never infer a vendor from a logo alone.
2. **Header fields**: invoice number, issue date, due date,
   currency, vendor address, bill-to address, PO reference if any.
   Each carries independent confidence. Dates in ambiguous format
   (DD/MM vs MM/DD without explicit region) → blank + escalate.
3. **Currency**: read the symbol AND any explicit ISO code. If only
   a `$` is present with no region context, leave currency blank
   and escalate — `$` is ambiguous across USD / CAD / AUD / etc.
4. **Line items** via `extract_table`: each item carries
   description, quantity, unit_price, line_total, tax_rate if any.
   Validate `qty * unit_price ≈ line_total` within 1¢ tolerance;
   surface mismatches in `_escalations` rather than recomputing.
5. **Totals**: subtotal, tax, shipping, grand_total. Validate
   `subtotal + tax + shipping ≈ grand_total` within 1¢. If the
   document totals do not reconcile, leave totals as read and
   escalate the reconciliation gap — do NOT "fix" them.
6. **PII discipline**: never echo full payment-card or bank-account
   numbers into the output; record only masked tails when those
   fields appear in scope.
7. **Refuse** handwritten invoices, partial scans (missing pages
   per the `Page N of M` header), and water-damaged documents.

Output shape:

```json
{
  "invoice": {
    "vendor": "ACME Co",
    "invoice_number": "INV-12345",
    "issue_date": "2026-05-01",
    "due_date": "2026-05-31",
    "currency": "USD",
    "subtotal": 1000.00,
    "tax": 100.00,
    "grand_total": 1100.00
  },
  "line_items": [
    {"description": "Widget A", "qty": 10, "unit_price": 50.00, "line_total": 500.00}
  ],
  "_confidence": {
    "vendor": 0.98, "invoice_number": 1.00,
    "issue_date": 0.95, "due_date": 0.90, "currency": 1.00
  },
  "_escalations": []
}
```
