# modules/efs

EFS filesystem for the `/data` mount (SQLite store + workspace state).

Provisions:
- One `aws_efs_file_system` (encrypted, generalPurpose, bursting).
- One mount target per private subnet.
- One EFS access point that exposes `/data` to the task under the distroless `nonroot`
  UID/GID (`65532`).
- An EFS security group that allows NFS (2049) only from the ECS service SG.
- A backup policy (`ENABLED`).
- Lifecycle policy → transition to IA after 30 days of no access.

Fargate cannot mount EBS, so EFS is the supported persistent store. Single-writer SQLite
+ a single-task service means there's only one writer ever; multi-AZ EFS is for graceful
task replacement, not for concurrent writers.
