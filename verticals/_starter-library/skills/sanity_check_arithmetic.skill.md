---
name: sanity_check_arithmetic
version: "0.1.0"
description: "Re-derives the numeric claims in a passage and flags any that don't add up — totals, percentages, ratios, unit conversions."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has a draft that contains arithmetic — a financial
summary, a metrics report, a price quote, a capacity estimate —
and wants the numbers checked before it goes out. Use any time
the question is "do these numbers add up?"

# Inputs

- `passage`: the text with numbers and the claims they support.
- `tolerance` (optional): relative error allowed (`"0.5%"`,
  `"$1"`, `"none"`). Default: `"0.5%"` for percentages, `"$1"`
  for currency.

# Output

A list of findings, each shaped as:
- **Claim** (the verbatim sentence under check).
- **Re-derivation** (how it was checked — what was summed,
  divided, converted).
- **Status** — `match`, `off_by_<delta>`, or `cannot_verify`
  (claim references data not in the passage).

A final `summary` line: `N claims checked, M match, K off, L
cannot_verify`. Return empty findings list if the passage
contains no arithmetic claims.

# Examples

- Quarterly report claiming "revenue up 12%" with prior + current
  numbers → re-derive, confirm or flag `off_by_2pp`.
- Cost estimate summing line items → check totals; flag any
  rounding drift past tolerance.

# Approval posture

Diagnostic. Skill flags; the operator (or author of the passage)
decides whether each off-by is a real error or an acceptable
rounding/typo. Skill does not edit the passage.

# Notes

- `cannot_verify` is a legitimate output, not a failure. The
  operator should treat it as "needs the underlying source data
  to check," not "ignore."
- Currency: never compare across currencies without an explicit
  exchange rate from the operator. If the passage mixes USD and
  EUR with no rate, flag `cannot_verify`.
- Round AFTER arithmetic, not before. Rounding inputs first is
  the leading source of false off-by flags.
