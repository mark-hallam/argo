# Skill: deduction eligibility

Use this skill when scanning a client's intake (W-2 line items,
1099s, receipts, prior-year return) for candidate deductions.
Output is a list of `flag_deduction_eligibility` actions, one per
candidate. Operator + client confirm; this skill never claims a
deduction on its own.

The rubric is conservative by design. The long tail of rare or
high-judgment deductions (R&D credits, exotic depreciation,
multi-state apportionment, foreign tax credit, energy credits with
phase-outs) is **explicitly out of scope** and triggers escalation
to the licensed preparer.

## When to apply

- Initial intake — scan for every rubric-matching candidate.
- Mid-season — re-scan when the client adds receipts, a 1099, or
  changes filing status.
- Pre-draft — confirm every candidate is `approved` or `rejected`
  before emitting `draft_return`.

## Common-deduction rubric

For each candidate below: rubric hit = the trigger condition; the
operator confirms the evidence; the preparer signs off.

1. **Standard deduction** — default unless itemized total clearly
   beats it. Trigger: every return.
2. **Mortgage-interest** — Form 1098 from a lender for a primary or
   second home. Trigger: client reports homeownership + mortgage.
3. **State and local tax (SALT)** — capped at $10k (MFJ + single).
   Trigger: state withholding on Box 17 + property-tax receipt.
4. **Charitable contributions** — substantiated cash + non-cash
   donations. Trigger: receipts > $250 per donee require contemp.
   acknowledgment; otherwise document the bank record.
5. **Medical expenses** — only the portion above 7.5% of AGI.
   Trigger: client surfaces unreimbursed medical receipts.
6. **Student-loan interest** — up to $2,500 above-the-line. Trigger:
   Form 1098-E from a lender; AGI within phase-out.
7. **HSA contribution** — above-the-line. Trigger: Box 12 code `W`
   on the W-2 already covers payroll-deducted HSA; only flag the
   non-payroll portion.
8. **Traditional IRA** — above-the-line; phased out if covered by
   employer plan (Box 12 code `D` or `E` signals coverage). Trigger:
   client confirms an IRA contribution; check phase-out.
9. **Roth IRA** — NOT a deduction. Trigger: never flag; note for
   the preparer if the client conflates the two.
10. **Student loan / educator expenses** — $300 above-the-line for
    K-12 educators; trigger on Box 14 / occupation code.
11. **Self-employment tax (1/2)** — half the SE tax is above-the-line.
    Trigger: any 1099-NEC > $400.
12. **SE health-insurance** — for the self-employed without an
    employer plan. Trigger: 1099 income + no W-2 health coverage.
13. **Childcare credit (CDCC)** — credit, not deduction; flag for
    the preparer with provider EIN. Trigger: dependents < 13 + paid
    childcare receipts.
14. **EITC** — credit; trigger on AGI band + qualifying-child test.
    Always escalate to preparer to confirm eligibility window.
15. **Energy / EV credits** — escalation only. Trigger: Form 5695
    or dealer EV statement → escalate.

## Escalation triggers (always preparer, never auto-flag)

- Home-office deduction (W-2 employees: never. SE only with strict
  exclusive-use test).
- Hobby vs business loss (Schedule C losses 3+ years running).
- Crypto / digital-asset transactions of any kind.
- Multi-state residency or non-resident state filings.
- Foreign income / FBAR / FATCA.
- Anything the client describes with "creative" or "aggressive".

## Evidence required to flag

A candidate without evidence is not flagged. Evidence shapes:

- A line-item from the W-2 (Box 12 code, Box 14 line).
- A 1098 / 1098-E / 1099 statement (cite issuer + amount).
- A receipt or bank record (cite date + payee + amount).
- A prior-year return line that recurs (cite year + line).

If the evidence is absent, ask the operator to follow up with the
client; do not flag on the client's verbal claim alone.
