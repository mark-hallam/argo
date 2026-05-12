---
name: tabulate_loose_data
version: "0.1.0"
description: "Turns a loose list of items into a clean table with inferred or operator-supplied columns — for quick comparison or downstream export."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has a pasted list — vendor names with prices,
candidates with notes, options with attributes — that wants
to be a table. Use any time the deliverable is "make a table
of <these things>" or "compare <these> in a grid."

# Inputs

- `items`: a list of strings, each one row's raw description.
- `columns` (optional): an ordered list of column names. If
  omitted, the skill proposes columns from item content and
  asks the operator to confirm before populating rows.
- `output_format`: `"markdown"` (default), `"csv"`, or `"json"`.

# Output

A table with one row per item, one column per `columns` entry.
Missing cells are explicitly empty (`""` in CSV/markdown,
`null` in JSON) — not "N/A" strings. A `_warnings` list
appended below/aside when extraction was uncertain for a row.

# Examples

- Items `<5 vendor blurbs>`, columns `[vendor, price, sla, region]`
  → 5×4 markdown table; price normalised to a numeric string
  with a currency column.
- Items `<10 candidate paragraphs>`, no columns supplied →
  skill replies with proposed columns `[name, current_role,
  years, location, source]` and waits for confirmation.

# Approval posture

Read-only. The operator decides what to do with the table.
Skill does not write to spreadsheets, databases, or files.

# Notes

- Never invent column values to "fill" the table. Empty is fine.
- If the operator hasn't supplied columns and the inferred set
  is ambiguous (e.g. two equally-plausible column sets), propose
  both and let the operator choose. Don't pick silently.
- Watch for unit drift across rows — "$1k" + "1500" + "two
  thousand" in a price column. Normalise to one unit and flag
  the conversion in `_warnings`.
