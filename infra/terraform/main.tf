# SKYCOT — AWS Fargate IaC root.
#
# Provisions a single-task ECS Fargate service running the `argo` gateway
# container, fronted by an ALB with TLS, backed by EFS for the SQLite store,
# with secrets sourced from AWS Secrets Manager.
#
# See infra/terraform/README.md + docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md for
# the operator command sequence.

provider "aws" {
  region = var.region

  default_tags {
    tags = merge(
      {
        Project   = var.project
        Env       = var.env
        ManagedBy = "terraform"
      },
      var.tags,
    )
  }
}

locals {
  name_prefix = "${var.project}-${var.env}"
}

module "network" {
  source = "./modules/network"

  name_prefix = local.name_prefix
  vpc_cidr    = var.vpc_cidr
  az_count    = var.az_count
}

module "ecr" {
  source = "./modules/ecr"

  name_prefix = local.name_prefix
}

module "efs" {
  source = "./modules/efs"

  name_prefix     = local.name_prefix
  vpc_id          = module.network.vpc_id
  private_subnets = module.network.private_subnet_ids
  service_sg_id   = module.alb.service_sg_id
  efs_access_path = "/data"
  efs_uid         = 65532
  efs_gid         = 65532
}

module "kms" {
  source = "./modules/kms"

  name_prefix = local.name_prefix
  region      = var.region
}

module "secrets" {
  source = "./modules/secrets"

  name_prefix = local.name_prefix
  kms_key_arn = module.kms.key_arn
}

module "iam" {
  source = "./modules/iam"

  name_prefix = local.name_prefix
  secret_arns = module.secrets.secret_arns
  kms_key_arn = module.kms.key_arn
}

module "alb" {
  source = "./modules/alb"

  name_prefix               = local.name_prefix
  vpc_id                    = module.network.vpc_id
  public_subnets            = module.network.public_subnet_ids
  domain_name               = var.domain_name
  route53_zone_id           = var.route53_zone_id
  front_with_cloudflare     = var.front_with_cloudflare
  env                       = var.env
  allow_open_origin_bringup = var.allow_open_origin_bringup
}

module "ecs" {
  source = "./modules/ecs"

  name_prefix         = local.name_prefix
  region              = var.region
  private_subnets     = module.network.private_subnet_ids
  service_sg_id       = module.alb.service_sg_id
  target_group_arn    = module.alb.target_group_arn
  alb_listener_arn    = module.alb.https_listener_arn
  image_repo_url      = module.ecr.repository_url
  image_tag           = var.image_tag
  task_cpu            = var.task_cpu
  task_memory         = var.task_memory
  log_retention_days  = var.log_retention_days
  execution_role_arn  = module.iam.execution_role_arn
  task_role_arn       = module.iam.task_role_arn
  efs_file_system_id  = module.efs.file_system_id
  efs_access_point_id = module.efs.access_point_id
  secret_arns         = module.secrets.secret_arns
  trust_xff           = var.front_with_cloudflare
  public_base_url     = var.public_base_url
  email_outbound      = var.email_outbound
  gateway_db_on_efs   = var.gateway_db_on_efs

  # DEC-056 durable DB ("B′"): the gateway entrypoint snapshots its local-disk DB
  # to the offsite backup bucket + restores on boot (fail-closed). Tie activation
  # to enable_offsite_backups (the bucket is provisioned by module.backup); empty
  # bucket => the wrapper is a no-op. The gateway task role gets scoped S3+KMS on
  # this bucket via the iam module (same deterministic name).
  db_snapshot_bucket   = var.enable_offsite_backups ? "${local.name_prefix}-backups" : ""
  db_snapshot_prefix   = var.gateway_db_snapshot_prefix
  db_snapshot_interval = tostring(var.gateway_db_snapshot_interval_seconds)
}

module "backup" {
  source = "./modules/backup"
  count  = var.enable_offsite_backups ? 1 : 0

  name_prefix         = local.name_prefix
  region              = var.region
  image_repo_url      = module.ecr.repository_url
  image_tag           = var.image_tag
  private_subnets     = module.network.private_subnet_ids
  service_sg_id       = module.alb.service_sg_id
  ecs_cluster_name    = module.ecs.cluster_name
  execution_role_arn  = module.iam.execution_role_arn
  efs_file_system_id  = module.efs.file_system_id
  efs_access_point_id = module.efs.access_point_id
  secret_arns         = module.secrets.secret_arns
  kms_key_arn         = module.kms.key_arn
  log_retention_days  = var.log_retention_days

  # DEC-056: the daily EFS-mounting backup task only makes sense when the DB is
  # actually on EFS. On local disk (B′) it would back up an empty EFS and fail
  # daily — disable it; the gateway's in-container snapshot loop is the durability
  # mechanism. The bucket + IAM + lifecycle in this module stay (B′ uses them).
  scheduled_backup_enabled = var.gateway_db_on_efs
}

module "observability" {
  source = "./modules/observability"

  name_prefix             = local.name_prefix
  alert_email             = var.alert_email
  alb_arn_suffix          = module.alb.alb_arn_suffix
  target_group_arn_suffix = module.alb.target_group_arn_suffix
  cluster_name            = module.ecs.cluster_name
  service_name            = module.ecs.service_name
}
