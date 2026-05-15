# AP Specialist Pack

The AP Specialist pack defines an accounts-payable worker for invoice intake, document extraction, three-way matching, exception routing, approval coordination, and ERP update preparation.

## Included Skills

- `process-invoice`: ingests invoice documents and extracts AP fields with confidence and evidence.
- `match-po`: compares invoice data against purchase orders and goods receipts.
- `flag-exception`: classifies AP exceptions and prepares evidence-backed exception notes.
- `route-for-approval`: creates approval packets and routes them over Slack before risky actions.
- `update-erp`: prepares approved ERP invoice updates for NetSuite, SAP, or Sage.

## Memory

The seed data includes vendor master rows, purchase orders, goods receipts, AP controls, exception routing rules, and approval history. These records support the sample five-invoice path and the deliberate mismatch fixture.

## Evaluation Coverage

`evals/golden.json` contains 25 fixtures across five invoice formats and common AP cases: clean three-way matches, low-confidence extraction, missing PO, duplicate invoice, tax review, currency mismatch, missing receipt, payment threshold routing, ERP draft creation, and one deliberate batch amount mismatch.

## Safety Model

The pack is dry-run by default. Human approval is required for ERP invoice updates, payment release, PO mismatch overrides, duplicate invoice overrides, and vendor-facing exception messages.

