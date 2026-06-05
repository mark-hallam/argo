# infra/terraform-bootstrap — remote-state backend

Creates the S3 bucket + DynamoDB lock table that the main stack
([`infra/terraform/`](../terraform/)) uses for **remote, locked terraform state**.

Local state on one laptop is fragile for a production deploy: state loss → orphaned/duplicate
AWS resources (cost leak), and no locking → concurrent applies corrupt state. S3 (versioned,
encrypted, public-access-blocked) + DynamoDB locking is the standard production posture. Cost
≈ pennies/month.

## Apply ONCE, before the main stack (operator runs)

```sh
cd infra/terraform-bootstrap
terraform init            # local state — this module bootstraps the remote backend
terraform apply           # creates skycot-prod-tfstate + skycot-prod-tflock
terraform output          # note state_bucket + lock_table
```

Back up this directory's `terraform.tfstate` securely (it's small + rarely changes). It is
the ONLY local state in the deploy.

## Then point the main stack at it (operator runs, once)

Add a `backend.tf` to `infra/terraform/` (it is intentionally NOT committed — backend blocks
can't use variables, so the bucket name is environment-specific):

```hcl
# infra/terraform/backend.tf  (operator creates after bootstrap)
terraform {
  backend "s3" {
    bucket         = "skycot-prod-tfstate"     # = bootstrap output state_bucket
    key            = "skycot/prod/terraform.tfstate"
    region         = "ap-southeast-2"
    dynamodb_table = "skycot-prod-tflock"      # = bootstrap output lock_table
    encrypt        = true
  }
}
```

Then migrate the main stack's existing local state into S3:

```sh
cd infra/terraform
terraform init -migrate-state   # prompts to copy local → S3; answer yes
```

After this, every `terraform apply` on the main stack reads/writes S3 with a DynamoDB lock —
safe across machines + concurrent runs.

## Validate locally (no AWS creds needed)

```sh
cd infra/terraform-bootstrap
terraform init -backend=false
terraform validate
terraform fmt -check
```

## Notes

- `prevent_destroy = true` on the bucket — a `terraform destroy` here will refuse, so you
  can't accidentally nuke the state store. To intentionally tear down, remove that lifecycle
  block first.
- The bucket name must be globally unique. Override `state_bucket_name` if `skycot-prod-tfstate`
  is taken.
- CI note for the disabled-GHA era: with remote state + locking, a future CI runner can safely
  `terraform plan` without racing the operator's local applies.
