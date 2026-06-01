---
id: DEC-032
date: 2026-06-01
status: active
supersedes: none
---

# DEC-032 — Next phase entered: Deployment Readiness (gated, pointable AWS product)

## Decision

The operator directed the program into its **next build phase — Deployment
Readiness** — *"Look at tasks beyond the MVP demo recording, what is high value
you can do"* → *"Enter the next phase of building"* → *"Continue high value next
phase work, and record we have entered the next phase."* The phase turns the
laptop demo into a **gated, pointable, AWS-deployable product**: a private URL
the operator can send investors / a waitlist to, on the ~$800 of AWS credits on
hand.

This is the **launch track** — it runs *alongside / after* the feature frontiers
([[DEC-030]] A→B→C→D), not as a new feature frontier. Division of labour: the
operator owns the AWS account, billing, and domain/DNS; the conductor preps
everything else (Dockerfiles, gating, runbook, prod-build fixes).

## Why

All six mission claims are LIVE and the demo is recordable, but the demo MP4 is
an operator-only artifact — there is nothing to *point people at*. The
highest-value beyond-demo work is a live, gated URL: the bridge from "demo on
Mark's Mac" to "a thing an investor can open." [[DEC-029]] truth-bounding still
governs every "done/live" claim in this phase.

## How (architecture decisions held)

- **Gated-demo topology = gateway on LOOPBACK behind nginx.** The gateway flips
  into production-auth mode (bearer-token required) the moment it binds a
  non-loopback address (`crates/argo-cli/src/handlers/gateway/routes_auth_production_mode.rs`:
  `is_production_mode = profile==production || bind-is-non-loopback`). For a
  gated demo we want the seeded data **viewable** (no login wall), so the
  gateway stays on `127.0.0.1` (NOT prod-mode → demo-mode shows real seeded
  data) and **nginx** provides public exposure + TLS + an HTTP **basic-auth
  gate** (private but pointable). Single small instance; Docker Compose with
  host networking.
- **The container bundles the gateway's on-disk runtime deps.** The gateway
  reads the pitch corpus (`skill-library/canonical-qa/...`), truth-boundary
  policies (`policies/truth-boundary/*.toml`), and provider manifests
  (`manifests/`) from disk at runtime (env overrides `ARGO_PITCH_CORPUS_DIR`,
  `ARGO_PITCH_POLICY_DIR`, `ARGO_MANIFEST_DIR`; data + key via `ARGO_HOME` +
  `SKYCOT_DB_KEY_BASE64`). The binary alone is not enough.
- **Prod SPA shows the demo via demo-mode, not auth-bypass.** `isAuthBypassed()`
  is FALSE in a production build (`NODE_ENV!=="production"`-gated), so the
  deployed SPA relies on `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` + `DEMO_LIVE=1` (baked
  at build) → it fetches **real seeded gateway data**. The nginx basic-auth is
  the privacy gate; demo-mode is the data path.
- **Scale-later path (documented, deferred):** App Runner / ECS Fargate for the
  gateway (non-loopback bind → prod bearer-token auth wired through), Amplify
  for the SPA, RDS Postgres, real magic-link auth (+ SES) — only when onboarding
  real users beyond the gated demo.
- **Truth-bounded ([[DEC-029]]):** "deployable/built" claims need current-turn
  evidence; the actual deploy is unverifiable until the operator's instance
  exists, so the kit is "prepped + locally-sound, deploy-verification pending."

## Outcome so far (2026-06-01, verified this session)

- **SPA production-build blocker FIXED + verified.** `next build` was failing
  (exit 1, masked earlier by a `| tail` pipe): the dynamic `/icon` +
  `/apple-icon` ImageResponse generators used `oklch()` colours Satori can't
  parse, and rendered an off-brand blue diamond duplicating the brand-correct
  static `icon.svg`. Deleted the `.tsx` generators (commit `72d8df0`) → `next
  build` green, `.next/standalone` produced. The SPA is now prod-buildable.
- **Deploy kit authored** (commit `80c04f0`): `docs/skycot/operator/DEPLOYMENT.md`
  (plan + runbook + cost + 6 honest refine-on-deploy items) + `deploy/`
  (`Dockerfile.gateway`, `Dockerfile.web`, `docker-compose.yml`, `nginx.conf`,
  `.env.deploy.example`, `.gitignore`).

## Relationship to the prior phase

The Sprint-2 platform build-out + feature frontiers A→B→C→D ([[DEC-030]]) remain
the feature track; Deployment Readiness is the parallel launch track. The voice
streaming spike (live two-way Portia) is built + backend-verified but pending
the operator's live mic grill — tracked separately, not part of this phase's
gate.

Related: [[DEC-030]] (feature frontiers), [[DEC-029]] (truth-bounded reporting),
[[DEC-021]] (voice/OpenAI — the gateway provider secret in the deploy env),
[[DEC-007]] (no theatre — the gated demo serves real seeded data).
