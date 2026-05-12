---
name: validate_against_schema
version: "0.1.0"
description: "Checks an object against a schema and returns a structured pass/fail with per-field reasons — no exceptions, no partial enforcement."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

After `extract_structured_data` or any other producer-skill,
the operator wants to confirm the output meets the contract
they need before downstream use. Use any time the request is
"is this <object> valid for <schema>?"

# Inputs

- `object`: the JSON object under test.
- `schema`: a JSON Schema (subset: `type`, `required`, `enum`,
  `format`, `minimum/maximum`, `pattern`, `items`, `properties`)
  or a TOML schema in the project's local dialect.
- `mode`: `"strict"` (every field validated) or `"required-only"`
  (only `required` fields enforced, others advisory).

# Output

```
{
  "valid": <bool>,
  "errors": [{"field": "...", "reason": "...", "got": "..."}, ...],
  "warnings": [{"field": "...", "reason": "...", "got": "..."}, ...]
}
```

Errors block in `strict`; warnings never block. Empty arrays
when nothing to flag.

# Examples

- Object missing a required `email` field → `valid: false`,
  one error citing `email`.
- Email field present but mis-formatted → error in `strict`,
  warning in `required-only` (the field is present).

# Approval posture

Pure check. No mutation. Safe to call freely. The caller decides
how to handle `valid: false` — retry, escalate, ask the operator.

# Notes

- Never silently coerce — `"5"` (string) for a `type: integer`
  field is an error, not a success. Coercion masks upstream
  bugs.
- Report ALL errors at once, not just the first. Stop-on-first
  is a CI anti-pattern that triples iteration cost for fixers.
- `format` checks (email, uri, date) are stricter than `type` —
  don't conflate them in error messages; cite the failing rule.
- If `schema` itself is malformed, return
  `valid: false, errors: [{field: "_schema", reason: "<why>"}]`
  rather than throwing.
