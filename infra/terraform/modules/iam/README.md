# modules/iam

ECS task IAM roles.

Provisions:
- **Task execution role** (`<prefix>-task-exec`) — the role the ECS agent assumes:
  - `AmazonECSTaskExecutionRolePolicy` (ECR pull + CloudWatch logs write).
  - A scoped `secretsmanager:GetSecretValue` policy over the exact secret ARNs the
    secrets module created (no wildcards).
- **Task role** (`<prefix>-task`) — the role the running container assumes. Empty today
  (the argo binary does NOT make AWS API calls). Future-proofs the wiring for S3 / SES / SSM
  surfaces; attach policies here when those land.
