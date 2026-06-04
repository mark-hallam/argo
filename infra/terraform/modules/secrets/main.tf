# Production secrets for the SKYCOT gateway.
#
# Every secret here is created with a PLACEHOLDER value ("REPLACE_ME"). The
# operator populates the real value out-of-band via:
#
#   aws secretsmanager put-secret-value --secret-id <id> --secret-string '<value>'
#
# `lifecycle.ignore_changes = [secret_string]` means a subsequent
# `terraform apply` does NOT clobber the populated value.
#
# WS5-3 (sprint-ws5-3-prod-secrets-bearer-token-bind-hardening) will harden
# this further (KMS key, recovery window, rotation).

locals {
  # Each secret declares:
  #  - id    : the SecretsManager id segment (final ARN is .../skycot-prod/<id>)
  #  - desc  : description shown in the SecretsManager console
  secret_specs = {
    bearer_token = {
      id   = "argo-gateway-bearer-token"
      desc = "ARGO_GATEWAY_BEARER_TOKEN — non-loopback bind admin bearer. openssl rand -hex 32."
    }
    workspace_root_key = {
      id   = "skycot-workspace-root-key-base64"
      desc = "SKYCOT_WORKSPACE_ROOT_KEY_BASE64 / ARGO_CREDENTIALS_MASTER_KEY — credential master key. 32 raw bytes, base64-encoded."
    }
    auth_jwt_secret = {
      id   = "skycot-auth-jwt-secret"
      desc = "SKYCOT_AUTH_JWT_SECRET — HMAC secret for __Host-session JWTs. 32+ random bytes."
    }
    stripe_secret_key = {
      id   = "stripe-secret-key"
      desc = "STRIPE_SECRET_KEY — Stripe live/test secret key."
    }
    stripe_webhook_secret = {
      id   = "stripe-webhook-secret"
      desc = "STRIPE_WEBHOOK_SECRET — endpoint signing secret for /api/v1/billing/webhook."
    }
    stripe_price_ids = {
      id   = "stripe-price-ids"
      desc = "JSON map { free, pro, enterprise } → Stripe price ids matching v1-tiers.toml."
    }
  }
}

resource "aws_secretsmanager_secret" "this" {
  for_each = local.secret_specs

  name        = "${var.name_prefix}/${each.value.id}"
  description = each.value.desc
}

resource "aws_secretsmanager_secret_version" "placeholder" {
  for_each = aws_secretsmanager_secret.this

  secret_id     = each.value.id
  secret_string = "REPLACE_ME"

  lifecycle {
    # Operator populates the real value via put-secret-value. Subsequent
    # terraform applies must not overwrite it back to the placeholder.
    ignore_changes = [secret_string]
  }
}
