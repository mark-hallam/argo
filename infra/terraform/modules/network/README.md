# modules/network

VPC + subnets + NAT for the Fargate deploy.

Provisions:
- One VPC sized at `vpc_cidr` (default `10.40.0.0/16`).
- Two public subnets (one per AZ, spans `az_count` AZs) — ALB lives here.
- Two private subnets (one per AZ) — Fargate tasks + EFS mount targets live here.
- One Internet Gateway, one NAT Gateway (single AZ — cost-conscious; multi-AZ NAT is a
  post-launch hardening), and route tables wiring private subnets out via NAT.

Cost note: a single NAT GW is ~$33/mo + per-GB egress. If 24/7 cost matters more than
single-AZ NAT failure tolerance, this is the right knob to revisit first.
