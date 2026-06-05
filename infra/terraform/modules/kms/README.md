# modules/kms

Customer-managed KMS key (CMK) that encrypts the production Secrets Manager entries.

Provisions:
- An `aws_kms_key` with **rotation enabled** and a 7-day deletion window.
- An `aws_kms_alias` (`alias/<prefix>-secrets`).
- A key policy that (a) grants the account root full admin so IAM policies can delegate
  use of the key and the operator is never locked out, and (b) allows Secrets Manager in
  this region to use the key via a `kms:ViaService` condition.

Why a CMK instead of the AWS-managed `aws/secretsmanager` key:
- Scoped decrypt — the task-execution role's `kms:Decrypt` is granted on this exact key ARN
  (see `modules/iam`), nothing broader.
- Key rotation under our control.
- CloudTrail visibility on every secret decrypt (the AWS-managed key hides this).

This is the enterprise-tier hardening posture for the governance moat. Cost: ~$1/mo + per-use.
