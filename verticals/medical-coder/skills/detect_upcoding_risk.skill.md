---
name: detect_upcoding_risk
description: Given a submitted (or proposed) claim plus underlying clinical documentation, flag CPT / ICD-10 lines selected at higher reimbursement than the documentation supports. Flags only; never auto-downcodes.
inputs:
  - claim_lines
  - encounter_documentation
  - payer_id
outputs:
  - codes_at_risk
  - recommended_action
  - flags
  - confidence
---

# Skill: Detect Upcoding Risk

Given a claim and the underlying clinical documentation, identify
patterns suggesting upcoding — codes that map to higher reimbursement
than the chart spans support. Flag-only; the supervising certified
coder reviews each finding before any downcode or amendment.

## Output

1. **Codes at risk** — list. For each line:
   - `code` — CPT / HCPCS / ICD-10 submitted on the claim.
   - `reason` — short category (`em_level_mismatch`,
     `complexity_gap`, `time_unit_overstated`, `unbundled_pair`,
     `modifier_overuse`, `dx_not_documented`).
   - `evidence_quote` — verbatim span from the documentation, with
     `[doc: <doc_id-or-page>, span: L<start>-L<end>]`. No paraphrase.
   - `severity` — `info` (documentation borderline; coder judgement),
     `advisory` (documentation supports a lower code; review),
     `urgent` (documentation does not support the submitted code at
     all; route to operator before further processing).

2. **Recommended action** — one of:
   - `send_as_is` — every finding is `info` and falls within
     documented coder judgement.
   - `downcode_to_<code>` — advisory finding with a single
     well-documented lower code; the supervising coder downcodes.
   - `request_clarification_from_provider` — documentation is
     ambiguous; provider query required before any code change.

3. **Confidence** — high (every finding cites an explicit span),
   medium (one or more spans `implied`), low (heuristic matches).

# Hard rules

- This skill never auto-downcodes. It flags. Operators downcode after
  reviewing the evidence — the routing always lands in human hands.
- Every `urgent` finding routes to the operator before the claim
  proceeds; the skill returns and waits, it does not advance.
- Findings without an `evidence_quote` are dropped, not surfaced.
- Treat E&M level mismatches and time-unit overstatements as
  load-bearing patterns — they are the most common upcoding vectors.

# PHI safety

- Synthetic data only. Production wiring must route real PHI through
  the approval-gated `release_unredacted_phi` action; this skill
  refuses to run against unredacted real PHI in any other context.
- `evidence_quote` spans are excerpted from documentation that is
  already inside the authorized envelope; the skill does not export
  them outside the coder review surface.
- Patient references use the synthetic encounter / patient ID only;
  Safe Harbor identifiers are redacted upstream via `redact_phi`.

I flag upcoding risk but never downcode — the certified coder signs
off before any adjustment goes to the payer.
