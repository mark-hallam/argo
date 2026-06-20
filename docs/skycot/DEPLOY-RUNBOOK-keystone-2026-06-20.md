# Deploy runbook — make the cold-stranger blocker fix LIVE (2026-06-20)

**Status:** main is deploy-complete at `3796bf0ba`. This is the conductor's execution
playbook for when the operator says "deploy the gateway." Prod is unchanged until
step 3 (`force-new-deployment`). **Operator-gated:** do NOT run step 3 without an
explicit go-ahead.

## What this deploy changes (vs current prod task-def `:9`)
- **Adds** the keystone fix (#2996): first-hire routes through the LLM classifier.
- **Adds** managed-keys-default onboarding (#2996), KPI persistence (#2996), 10% markup (#2996).
- **Keeps** everything `:9` has: durable-DB entrypoint, genesis cost-cap (#2997 now on main too), suspend-lockout, Stripe, SMTP, ANTHROPIC_API_KEY.
- **Sets one new env var:** `ARGO_GENESIS_LLM_CLASSIFIER=1` — this is what actually turns the blocker fix ON (the code is a no-op without it).

## Pre-deploy facts (verified 2026-06-20)
- ECR gateway repo: `123456789012.dkr.ecr.ap-southeast-2.amazonaws.com/skycot-prod-argo`
- Current live: task-def `skycot-prod-argo:9`, image `:genesis-ratelimit-20260619`, **ARM64**, cpu 1024 / mem 2048.
- Service: `skycot-prod-argo` on cluster `skycot-prod-cluster`, region `ap-southeast-2`.
- Prod already has: `ANTHROPIC_API_KEY` (secret), `ARGO_GATEWAY_PROFILE=production` (so the spend cap bites — confirmed), `ARGO_GENESIS_LLM_PROVIDER` (anthropic).

## Step 1 — build the ARM64 image from main + push  [conductor, non-destructive]
Build **natively on Apple Silicon** (the 2026-06-18 session moved off flaky QEMU amd64 → native arm64, ~8 min). Root `Dockerfile` carries the durable-DB entrypoint.
```
TAG=keystone-$(git -C ~/code/argo rev-parse --short origin/main)   # e.g. keystone-3796bf0ba
ECR=123456789012.dkr.ecr.ap-southeast-2.amazonaws.com/skycot-prod-argo
aws ecr get-login-password --region ap-southeast-2 | docker login --username AWS --password-stdin 123456789012.dkr.ecr.ap-southeast-2.amazonaws.com
# build from a clean main checkout/worktree at 3796bf0ba:
docker build --platform linux/arm64 -t $ECR:$TAG .
docker push $ECR:$TAG
```

## Step 2 — register task-def :10 (clone :9 + new image + the flag)  [conductor, non-destructive]
Clone `:9`, swap the image to `$ECR:$TAG`, add `ARGO_GENESIS_LLM_CLASSIFIER=1` to the
`environment` array. Keep every other env + every secret unchanged. Register → get `:10`.
(Registering a task-def does NOT change what's running.)

## Step 3 — deploy  [OPERATOR GO-AHEAD REQUIRED]
```
aws ecs update-service --cluster skycot-prod-cluster --service skycot-prod-argo \
  --task-definition skycot-prod-argo:10 --force-new-deployment --region ap-southeast-2
# wait for the new task to reach RUNNING + the target group healthy
```

## Step 4 — smoke (proves the blocker is fixed)  [conductor]
1. `curl https://api.skycot.com/health` → 200, schema ok.
2. **The blocker test** — a cold-stranger plain-English first-hire that previously hard-422'd:
   sign up → describe a teammate in a sentence that has NO keyword match
   (e.g. *"help me stay on top of my freelance clients"*) → expect a teammate **created**
   (not `genesis_plan_failed`). This is the camera-honest proof the fix works.
3. Cost-cap still bites: burst the anonymous `/api/v1/genesis` SSE > cap → expect 429.

## Rollback (if smoke fails)
```
aws ecs update-service --cluster skycot-prod-cluster --service skycot-prod-argo \
  --task-definition skycot-prod-argo:9 --force-new-deployment --region ap-southeast-2
```
`:9` is the known-good (current) definition — one command back.

## Sequencing note
Best done **after** Cloudflare is proxying both domains (perimeter up) and the Anthropic
$500 cap is set (blast-radius bound), since enabling the LLM classifier increases provider
spend on the (capped, authenticated) onboarding path. Not required, but cleaner.
