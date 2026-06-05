# infra-rebase-and-land-cloudflare-pr-2692 (Windows / infra lane — non-cargo)

**Owner**: windows or claude (non-cargo: git + terraform only; no Rust build).
**Depends on**: the 8 merged infra PRs (#2695/#2685/#2688/#2693/#2691/#2689/#2686/#2707, all landed 2026-06-05).
**Authored against**: origin/main `da38d892a` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-rebase-and-land-cloudflare-pr-2692`.

## Why
8 of the 9 built infra PRs merged in a train. **PR #2692** (`infra(cloudflare) — front_with_cloudflare toggle: lock origin to Cloudflare`) was the one that went **CONFLICTING** — it overlaps the now-merged ALB/ECS terraform changes on the shared module files. Rebase it, resolve the (additive) terraform conflict, and push so the conductor can merge it.

## Task (non-cargo — git + terraform)
1. `gh pr checkout 2692`.
2. `git fetch origin main && git rebase origin/main`.
3. Resolve the conflicts. They are on the shared terraform modules (`infra/terraform/main.tf`, `modules/alb/*`, `modules/ecs/*`, `variables.tf`, `outputs.tf`). They are almost certainly **additive on both sides** — #2692 adds the `front_with_cloudflare` toggle + Cloudflare-IP ingress restriction; the merged PRs added other variables/resources to the same files. **Keep BOTH sides'** additions; do not drop either. Re-read the final files to confirm the toggle's ingress rule and the merged PRs' resources both survive.
4. Validate: `terraform fmt -check` + `terraform validate` in `infra/terraform/` (and `bash scripts/validate-infra.sh` if present — landed via #2707). NO `terraform apply` (that's the operator's live step).
5. Push to the #2692 branch (`git push --force-with-lease`).

## Acceptance
PR #2692 is MERGEABLE again on current main; `terraform validate` passes; the `front_with_cloudflare` toggle (default-off) + the Cloudflare-IP origin lock are intact alongside the merged ALB/ECS resources. Conductor merges after.

## PR
Updates existing PR #2692. No new PR. Conductor adversarial-reviews the perimeter change before merge (WS5).
