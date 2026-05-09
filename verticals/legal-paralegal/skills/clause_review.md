# Skill: clause-pattern recognition

Use this skill when running `review_contract` to bucket each clause
in a contract into `standard` / `minor_concern` / `major_concern`.
The output drives whether `flag_clause_concern` fires for the clause.

## When to apply

- The contract is on file in `inventory.json` (the source of truth
  for `contract_id`, `type`, `counterparty`, `effective_date`).
- You are running `review_contract` against the standard checklist.
- You are NOT being asked for an opinion on whether the clause is
  acceptable — that decision belongs to the supervising attorney via
  `flag_clause_concern`.

## Standard checklist

Walk every contract against the following clause headings, in order.
Note each as present / absent and bucket it:

1. **Parties + recitals** — names, addresses, capacity. Bucket
   `major_concern` if either party is unnamed.
2. **Term + termination** — effective date, end date or trigger,
   termination-for-cause / termination-for-convenience. Bucket
   `minor_concern` if termination-for-cause is one-way.
3. **Governing law + venue** — jurisdiction. Bucket `minor_concern`
   if the venue is inconvenient relative to the client's location.
4. **Liability cap** — explicit cap (typically prior 12 months of
   fees). Bucket `major_concern` if uncapped or unspecified.
5. **Indemnity** — who indemnifies whom, for what claims, with
   what carve-outs. Bucket `major_concern` if indemnity runs only
   one way against the client, or if there are no carve-outs.
6. **IP assignment** — who owns work product, when assignment
   vests. Bucket `major_concern` if IP vests on signature rather
   than on payment, or if assignment language is absent for an
   SOW/MSA/employment contract.
7. **Confidentiality** — mutual or one-way, term, carve-outs.
   Bucket `minor_concern` if one-way against the client.
8. **Payment terms** — invoicing cadence, late-payment terms,
   currency. Bucket `minor_concern` if late-payment interest is
   above the firm's standard rate.

## What to write

For each `major_concern` clause, the review output carries:

1. **Clause heading** — verbatim from the contract.
2. **Concern category** — one of `liability` / `indemnity` / `ip` / `other`.
3. **Verbatim text** — quote the clause without paraphrase.
4. **Why flagged** — one-sentence observation, *not* an opinion.
   Good: "Liability cap is unspecified; counterparty's exposure is
   open-ended." Bad: "This is unenforceable in California."

## What NOT to write

- No opinions on enforceability, jurisdiction-shopping, or whether
  the client should accept the clause.
- No paraphrased clause text — always quote verbatim so the attorney
  reviews the actual language.
- No suggestions about counterparty motivation or what they would
  accept in negotiation.
- No "I recommend", "you should", or "the right move is" language —
  flag the clause and let the attorney decide.

## Example (MSA-2026-0044, liability bucket)

```
Clause heading: Section 8.1 — Limitation of Liability
Concern category: liability
Verbatim text: "Each party's liability under this Agreement shall
                be unlimited."
Why flagged: Liability cap is unspecified; the client's exposure
             is open-ended. Routing to S. Hallam for negotiation.
```
