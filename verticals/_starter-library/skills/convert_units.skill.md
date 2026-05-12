---
name: convert_units
version: "0.1.0"
description: "Converts a value from one unit to another (length, mass, volume, currency, energy, data, time) with explicit precision and rate-source for currency."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

Any time the operator (or another skill) needs `<value> <unit
A>` expressed as `<unit B>`. Use across reports, customer-facing
docs, regulatory filings, vendor comparisons — wherever
unit-mismatch is the bug to prevent.

# Inputs

- `value`: numeric input (string or number; never converted from
  ambiguous prose).
- `from_unit`: the source unit (`"mile"`, `"USD"`, `"GiB"`,
  `"hour"`).
- `to_unit`: the target unit.
- `precision`: significant figures to keep (default 3).
- `currency_rate_source` (currency only): `"ecb-daily"`,
  `"operator-supplied:<rate>"`, etc. Required for currency
  conversion — see Notes.

# Output

A `{value, unit, precision, source}` object. For currency, an
extra `rate, rate_timestamp, rate_source` block. For unambiguous
conversions (length/mass/etc.), `source` is `"definition"`.

# Examples

- 5 miles → km, precision 4 → `8.047 km`.
- 1000 USD → EUR, source `ecb-daily` → `<EUR amount>` with
  rate + ECB date stamp.
- 2 GiB → MB, precision 3 → `2150 MB` (with note about
  GiB-vs-GB binary/decimal distinction).

# Approval posture

Pure computation for non-currency. Currency conversion requires
an explicit rate source from the operator/runtime; skill refuses
to fabricate a rate.

# Notes

- Binary vs decimal units (GiB vs GB, MiB vs MB) are NOT
  interchangeable. Always preserve the distinction; flag in
  `note` when converting across.
- Currency: never invent or "remember" exchange rates. If no
  `currency_rate_source` is supplied, return an error message
  and stop.
- Temperature: linear conversions only (no temperature
  differences vs absolute temperatures confusion — be explicit
  about which the operator wants if ambiguous).
- Time: be careful with "month" and "year" — define them in
  the response (calendar month vs 30 days, year vs 365.25).
