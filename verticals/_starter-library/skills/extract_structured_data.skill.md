---
name: extract_structured_data
version: "0.1.0"
description: "Extracts a target set of fields from unstructured input (email, PDF text, transcript) into a strict JSON shape — explicit-null on misses, no guessing."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has unstructured text and a known schema of fields
they want pulled out — invoice header, contact card, intake form
data, a parsed-out address. Use any time the request is
"give me <these fields> from <this blob>."

# Inputs

- `source_text`: the unstructured input.
- `target_fields`: a list of field names AND each field's
  expected type (`string`, `integer`, `currency_amount`,
  `iso_date`, `email`, `phone_e164`, etc.).
- `strict` (optional, default true): if true, return `null` for
  fields not found verbatim in source; if false, allow modest
  inference (e.g. assembling a full name from "first" + "last").

# Output

A flat JSON object: each `target_fields` key present, value
either the typed extraction or `null`. A sibling `_evidence`
object maps each filled key to the source quote it came from.

# Examples

- Source `<email signature block>`, fields `[name, title,
  company, email, phone]` → object with each field filled OR
  null; phone normalised to E.164.
- Source `<invoice PDF text>`, fields `[invoice_number,
  total_amount, due_date]` → strict typed values; `_evidence`
  cites the literal substring.

# Approval posture

Read-only. Safe to call without confirmation. Downstream skills
or operator decide whether the extraction is good enough to act
on; this skill never writes the data anywhere.

# Notes

- Never invent values. Missing field → `null`, not "Unknown" or
  "N/A".
- Currency: emit `{amount: <decimal_string>, currency: "USD"}`
  shape, not a float — float math on money is the leading data
  bug.
- Dates: ISO-8601 only. If the source date is ambiguous
  (`03/04/2026` — March or April?), return null and add to
  `_warnings`.
- `_evidence` is mandatory for every non-null field so a human
  can verify the extraction without re-reading the source.
