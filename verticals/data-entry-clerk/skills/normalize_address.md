---
name: normalize_address
description: Given a free-text address string, output a structured street1/street2/city/state/postal/country record with per-field confidence and a country-specific format flag.
inputs:
  - raw_address
  - country_hint
outputs:
  - address
  - _confidence
  - _format_flag
  - _escalations
---

# Skill: Normalize Address

Given a free-text address, produce a structured record. The shape is
the same across countries; the format flag declares which national
format the address most plausibly fits.

1. **Detect country first.** Use the `country_hint` if supplied; else
   infer from postal-code shape, state/province token, country line.
   If the country cannot be detected with confidence ≥ 0.70, leave
   country blank, escalate, and use a generic split.
2. **Split into fields**: `street1`, `street2`, `city`, `state`,
   `postal`, `country`. Each cell carries an independent confidence.
3. **Validate per-country format.** Examples:
   - US: 5- or 9-digit ZIP; 2-letter state code from the canonical set.
   - GB: alphanumeric postcode (e.g. `SW1A 1AA`); county is optional.
   - AU: 4-digit postcode; state from {NSW, VIC, QLD, WA, SA, TAS, ACT, NT}.
   - CA: alphanumeric postal (`A1A 1A1`); province from canonical set.
   Failures here lower confidence and surface in `_escalations`, but
   never silently rewrite the value.
4. **Apply vendor aliases.** Check `vendor_aliases` memory for a
   canonical form of the input; if a match exists with confidence ≥
   0.90, prefer the alias and note the substitution in `_escalations`
   for audit visibility.
5. **Set `_format_flag`** to the detected country code (ISO 3166-1
   alpha-2) or `unknown`.
6. **Never invent missing components.** A street number with no
   street name → leave street1 blank, escalate. A city with no
   postal → keep city, leave postal blank, escalate.

Output shape:

```json
{
  "address": {
    "street1": "10 Downing St",
    "street2": "",
    "city": "London",
    "state": "",
    "postal": "SW1A 2AA",
    "country": "GB"
  },
  "_confidence": {
    "street1": 0.95, "street2": 1.00, "city": 0.95,
    "state": 0.00, "postal": 1.00, "country": 0.92
  },
  "_format_flag": "GB",
  "_escalations": [
    {"field": "state", "reason": "no county in raw; UK addresses often omit"}
  ]
}
```
