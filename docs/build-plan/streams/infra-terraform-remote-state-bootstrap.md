# infra-terraform-remote-state-bootstrap

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: nothing — standalone root module, independent of the main stack (infra/terraform/).
**Authored against**: origin/main `54d3f5395` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-terraform-remote-state-bootstrap`.

## Why

The AWS deploy runbook (WS5-1) uses **local terraform state** on the operator's laptop. For a
production deploy that's fragile + a reliability risk (priority #1):
- State loss → orphaned/duplicate AWS resources on re-apply (cost leak on the $800 credit).
- No locking → concurrent applies (or a CI runner + a manual apply) corrupt state.

The standard production posture is **remote, locked state**: an S3 bucket (versioned,
encrypted, public-access-blocked) + a DynamoDB lock table. Cost ≈ pennies/month.

## What

A standalone `infra/terraform-bootstrap/` root module (NOT part of the main stack — it's the
chicken-and-egg that creates the backend the main stack then uses):
- S3 bucket (`skycot-prod-tfstate`, overridable): versioning ON, SSE-AES256, public access
  blocked, `prevent_destroy` so it can't be accidentally nuked.
- DynamoDB lock table (`skycot-prod-tflock`, PAY_PER_REQUEST, `LockID` hash key).
- README: apply-once sequence, the `backend "s3"` block to paste into the main stack, and the
  `terraform init -migrate-state` to move existing local state into S3.

## May-touch
- `infra/terraform-bootstrap/**` (NEW — standalone module).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip).

Does NOT touch the main stack (`infra/terraform/`) — the operator adds the uncommitted
`backend.tf` after bootstrap (backend blocks can't use variables, so it's environment-local).
No Rust.

## Acceptance criteria
- [ ] `cd infra/terraform-bootstrap && terraform init -backend=false && terraform validate` exits 0.
- [ ] `terraform fmt -check` exits 0.
- [ ] Bucket has versioning + SSE + public-access-block + `prevent_destroy`.
- [ ] DynamoDB table has `LockID` hash key, PAY_PER_REQUEST.
- [ ] README documents the backend block + `-migrate-state`.
- [ ] No file >500 LOC; no Rust changed.

## Out of scope
- A live apply (operator-gated).
- Adding `backend.tf` to the main stack (operator-local; backend can't use vars).
- State for the DNS / WAF standalone modules (they're tiny + operator-applied; can adopt the
  same bucket later with distinct keys if desired).

## Parallel-surface check
- `rg 'tfstate|dynamodb_table.*lock|terraform-bootstrap' infra/` — none before this; new.
- Independent of the main `infra/terraform/` stack (separate root, separate state).

## Source
Production reliability (priority #1) — the WS5-1 runbook's local-state note ("post-launch we
migrate to an S3 backend"). Brought forward to launch.
