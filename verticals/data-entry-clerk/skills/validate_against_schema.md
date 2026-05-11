---
name: validate_against_schema
description: Given a parsed row and a target schema, flag missing required fields, type mismatches, and unknown enum values; never silently coerces.
inputs:
  - row
  - target_schema_id
outputs:
  - validation
  - _escalations
---

# Skill: Validate Against Schema

Run after any extract / parse skill. Compares a parsed row against
the operator-defined target schema and emits a validation report.
Never mutates the row.

1. **Resolve the schema.** Look up `target_schema_id` in
   `target_schemas` memory or `inventory.json`. If unknown, refuse —
   validation is undefined without a schema.
2. **Required-field check.** For each required field, flag missing
   (`value == null`) and blank (`value == ""`) cases. Surface them
   in `validation.missing_required`.
3. **Type check.** Each value must match the declared column type
   (`int`, `decimal`, `date(YYYY-MM-DD)`, `iso_country_2`,
   `currency_3`, `enum<...>`, `string`). Type mismatches go in
   `validation.type_mismatches` with the expected and actual types.
4. **Enum check.** Values for enum columns must be in the declared
   set. Surface unknown enum values in
   `validation.unknown_enum_values` — never auto-correct to the
   nearest neighbor.
5. **Confidence floor.** Any field with `_confidence < 0.70` goes
   into `validation.low_confidence`. These rows always route through
   the `low_confidence_field` escalation gate.
6. **Aggregate `ok`.** Set `validation.ok = true` only if all of:
   no missing required, no type mismatches, no unknown enums, no
   low-confidence fields. Else `ok = false` and the row must be
   reviewed by a human before commit.

NEVER silently coerce a type mismatch. NEVER substitute a "close"
enum value. The skill is read-only on the row by design.

Output shape:

```json
{
  "validation": {
    "ok": false,
    "missing_required": ["due_date"],
    "type_mismatches": [
      {"field": "qty", "expected": "int", "actual": "string", "value": "ten"}
    ],
    "unknown_enum_values": [
      {"field": "currency", "value": "$", "allowed": ["USD", "EUR", "GBP", "AUD"]}
    ],
    "low_confidence": [
      {"field": "invoice_number", "confidence": 0.55}
    ]
  },
  "_escalations": [
    {"reason": "missing_required:due_date"},
    {"reason": "type_mismatch:qty"},
    {"reason": "low_confidence:invoice_number"}
  ]
}
```
