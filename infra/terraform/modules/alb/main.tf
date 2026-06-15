# front_with_cloudflare = true (the secure default) restricts ingress to
# Cloudflare's ranges so the origin can't be hit directly (every Cloudflare WAF +
# rate-limit + bot rule becomes bypass-proof) AND the gateway can safely trust
# X-Forwarded-For for per-IP rate limiting. false opens it to the internet — valid
# ONLY for the transient pre-DNS bring-up smoke, gated below by the precondition.
locals {
  alb_ingress_v4 = var.front_with_cloudflare ? var.cloudflare_ipv4_cidrs : ["0.0.0.0/0"]
  alb_ingress_v6 = var.front_with_cloudflare ? var.cloudflare_ipv6_cidrs : ["::/0"]

  # The origin is internet-open iff the resolved ingress is the wildcard.
  alb_origin_open = contains(local.alb_ingress_v4, "0.0.0.0/0") || contains(local.alb_ingress_v6, "::/0")
}

resource "aws_security_group" "alb" {
  name        = "${var.name_prefix}-alb-sg"
  description = "ALB - public ingress on 80 and 443 (Cloudflare-only by default; open only for a deliberate pre-DNS bring-up)."
  vpc_id      = var.vpc_id

  # Hard-gate: a prod plan/apply FAILS LOUD if the resolved origin is open to the
  # whole internet (0.0.0.0/0 / ::/0), unless the operator has consciously set the
  # bring-up escape hatch. This makes an internet-open prod origin impossible to
  # reach SILENTLY — it bypasses Cloudflare WAF + rate-limits + the LLM-cost block.
  lifecycle {
    precondition {
      condition     = var.allow_open_origin_bringup || var.env != "prod" || !local.alb_origin_open
      error_message = "The prod ALB security group is open to 0.0.0.0/0. Set front_with_cloudflare=true so the origin is Cloudflare-CIDR-only before go-live, or allow_open_origin_bringup=true ONLY for the transient pre-DNS bring-up smoke (revert before go-live)."
    }
  }

  dynamic "ingress" {
    for_each = toset([80, 443])
    content {
      description      = "HTTP(S) ingress on ${ingress.value}"
      from_port        = ingress.value
      to_port          = ingress.value
      protocol         = "tcp"
      cidr_blocks      = local.alb_ingress_v4
      ipv6_cidr_blocks = local.alb_ingress_v6
    }
  }

  egress {
    description      = "Forward to the service in private subnets"
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  tags = {
    Name = "${var.name_prefix}-alb-sg"
  }
}

# Service SG — ingress only from the ALB SG on the container port.
# Egress all (the gateway calls provider APIs + secrets + ECR via NAT).
resource "aws_security_group" "service" {
  name        = "${var.name_prefix}-svc-sg"
  description = "Fargate service SG - accepts only from ALB SG on container port."
  vpc_id      = var.vpc_id

  ingress {
    description     = "ALB -> service"
    from_port       = var.container_port
    to_port         = var.container_port
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  egress {
    description = "All egress (provider APIs + ECR pull via NAT + Secrets Manager via VPC endpoint or NAT)."
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.name_prefix}-svc-sg"
  }
}

resource "aws_lb" "this" {
  name               = "${var.name_prefix}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = var.public_subnets

  enable_deletion_protection = true
  idle_timeout               = 60

  tags = {
    Name = "${var.name_prefix}-alb"
  }
}

resource "aws_lb_target_group" "this" {
  name                 = "${var.name_prefix}-tg"
  port                 = var.container_port
  protocol             = "HTTP"
  vpc_id               = var.vpc_id
  target_type          = "ip"
  deregistration_delay = 30

  health_check {
    enabled             = true
    path                = var.health_check_path
    protocol            = "HTTP"
    matcher             = "200-299"
    interval            = 15
    timeout             = 5
    healthy_threshold   = 2
    unhealthy_threshold = 3
  }

  tags = {
    Name = "${var.name_prefix}-tg"
  }
}

resource "aws_acm_certificate" "this" {
  domain_name       = var.domain_name
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }

  tags = {
    Name = var.domain_name
  }
}

# Route53 validation record — created only when the operator has Route53 manage
# the public zone. When DNS is on Cloudflare (the sprint default), the operator
# pastes the validation CNAMEs into Cloudflare manually per the runbook.
resource "aws_route53_record" "validation" {
  for_each = var.route53_zone_id == "" ? {} : {
    for dvo in aws_acm_certificate.this.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  zone_id         = var.route53_zone_id
  name            = each.value.name
  type            = each.value.type
  ttl             = 60
  records         = [each.value.record]
}

resource "aws_acm_certificate_validation" "this" {
  count                   = var.route53_zone_id == "" ? 0 : 1
  certificate_arn         = aws_acm_certificate.this.arn
  validation_record_fqdns = [for r in aws_route53_record.validation : r.fqdn]
}

resource "aws_lb_listener" "http_redirect" {
  load_balancer_arn = aws_lb.this.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = "redirect"

    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.this.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = aws_acm_certificate.this.arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.this.arn
  }
}
