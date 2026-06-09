# ws5-consolidate-deploy-docker-onto-canonical-gateway-stack

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ws5-consolidate-deploy-docker-onto-canonical-gateway-stack`.

## Why (two divergent Docker stacks — one ships a policy-less gateway, VERIFIED 2026-06-09)
There are TWO Docker stacks for the same `argo` gateway, and the LEGACY one would ship a gateway MISSING its truth-boundary policies + pitch corpus — i.e. a broken, NO-THEATER-violating gateway. An operator who follows the wrong README deploys it. This is drift-cleanup: subtract the legacy stack before it bites.

VERIFIED live (read 2026-06-09):
- **Canonical (correct).** `deploy/Dockerfile.gateway` builds on `rust:1.83-slim-bookworm`, and crucially bundles the gateway's on-disk runtime deps: `COPY … /src/manifests`, `/src/policies`, `/src/skill-library` (`:33-35`) with `ARGO_MANIFEST_DIR` / `ARGO_PITCH_CORPUS_DIR` (`skill-library/canonical-qa/corpora/skycot`) / `ARGO_PITCH_POLICY_DIR` (`policies/truth-boundary`) env (`:36-39`). Binds loopback `127.0.0.1:7400` on purpose so it stays OUT of production-auth mode and nginx is the public gate (`:42-43`). Referenced by `deploy/docker-compose.yml:14` (`dockerfile: deploy/Dockerfile.gateway`), which is the documented bring-up in `deploy/docker-compose.yml:8`.
- **Legacy (broken).** `deploy/docker/Dockerfile.argo:11` builds on the OLD `rust:1.75-slim`; `:38` `EXPOSE 8080`; `:49-50` `ENTRYPOINT … --bind 0.0.0.0 --port 8080 --insecure-bind`. It copies `manifests` but does **NOT** copy `policies` or `skill-library` (`:20-24` only copies Cargo manifests + `crates`/`tools`/`manifests`/`mcp-servers`) — so the running gateway has no truth-boundary policy dir and no pitch corpus, and binds a non-loopback address (flipping it into production-auth mode). This is the gateway the legacy compose builds: `deploy/docker/docker-compose.yml:39` (`dockerfile: deploy/docker/Dockerfile.argo`).
- **READMEs still point operators at the broken stack.** `deploy/k8s/README.md:51` literally builds it: `docker build -f deploy/docker/Dockerfile.argo -t argo:dev .`; `deploy/k8s/README.md:7-9,29-31` and `deploy/vm/README.md:6,175` send operators to `deploy/docker/` as "the single-machine path". The whole `deploy/docker/` dir (`Dockerfile.argo`, `Dockerfile.web`, `docker-compose.yml`, `docker-compose.prod.yml`, `.env.example`, `README.md`) is the legacy parallel stack to the canonical root-level `deploy/Dockerfile.gateway` + `deploy/docker-compose.yml`.

## Grep-before-write
- `rg -n 'Dockerfile\.argo' deploy/` — the LIVE references to remove/repoint: `deploy/k8s/README.md:51`, `deploy/docker/docker-compose.yml:39` (plus `deploy/docker/README.md`). (Repo-wide there are also historical hits under `docs/build-plan/streams/` + `docs/skycot/audit/` — those are HISTORICAL brief/audit text, NOT live deploy config; leave them, or add a one-line "superseded by deploy/Dockerfile.gateway" note only if trivially adjacent.)
- `rg -n 'deploy/docker/|docker-compose' deploy/k8s/README.md deploy/vm/README.md deploy/docker/README.md` — every README pointer at the legacy dir to reconcile onto the canonical `deploy/Dockerfile.gateway` + `deploy/docker-compose.yml`.
- `rg -n 'Dockerfile\.gateway|deploy/docker-compose\.yml' deploy/` — confirm the canonical stack is what compose + DEPLOYMENT.md already use (do NOT create a third stack).
- No Rust surface touched — `rg 'pub (struct|enum|fn)'` not applicable; this is a deploy-config consolidation. Confirm no script consumes the legacy path: `rg -n 'deploy/docker/(Dockerfile\.argo|docker-compose)' scripts/ Makefile .github/ deploy/smoke/`.

## Fix
1. **Subtract the legacy stack.** Delete `deploy/docker/Dockerfile.argo`. Because the rest of `deploy/docker/` (`Dockerfile.web`, `docker-compose.yml`, `docker-compose.prod.yml`, `.env.example`, `README.md`) only exists to drive that broken Dockerfile against port 8080 / `0.0.0.0`, delete the whole `deploy/docker/` directory in favour of the canonical `deploy/Dockerfile.gateway` + `deploy/Dockerfile.web` + `deploy/docker-compose.yml`. (If a reviewer wants a soft landing instead of a hard delete, replace `deploy/docker/README.md` with a single deprecation pointer — "MOVED: use `deploy/docker-compose.yml` (builds `deploy/Dockerfile.gateway`) — see `docs/skycot/operator/DEPLOYMENT.md`" — and delete only `Dockerfile.argo` + the two legacy compose files. Either way `Dockerfile.argo` must not survive as a buildable target.)
2. **Reconcile the k8s README.** In `deploy/k8s/README.md`, repoint the image-build steps (`:51-53`, `:7-9`, `:29-31`) from `deploy/docker/Dockerfile.argo` / `deploy/docker/` to `docker build -f deploy/Dockerfile.gateway -t argo:dev .` and `docker build -f deploy/Dockerfile.web -t argo-web:dev .`, and update the "Pairs with `deploy/docker/`" prose to "the canonical compose stack (`deploy/docker-compose.yml`)".
3. **Reconcile the vm README.** In `deploy/vm/README.md`, update the two "for docker-compose see `deploy/docker/`" pointers (`:6`, `:175`) to `deploy/docker-compose.yml`.
4. Additive/clean only: no new Dockerfile, no new compose file, no new pub type, no migration. Consumer of the canonical stack stays `deploy/docker-compose.yml` (already references `deploy/Dockerfile.gateway`) + the k8s/vm READMEs after this repoint.

## Acceptance (truth-bounded)
1. **Checkable verifier — no LIVE legacy reference:** after the fix, `rg -n 'Dockerfile\.argo' deploy/` returns ZERO matches (or, if the soft-deprecation option is taken, ONLY a single non-buildable deprecation-pointer line in `deploy/docker/README.md`, never a `docker build -f …` or compose `dockerfile:` line).
2. **Checkable verifier — single canonical stack:** `rg -n 'dockerfile:' deploy/*.yml deploy/**/*.yml` shows the only gateway image built is `deploy/Dockerfile.gateway`; `rg -n 'deploy/docker/' deploy/k8s/README.md deploy/vm/README.md` returns ZERO matches (both READMEs now point at `deploy/Dockerfile.gateway` / `deploy/docker-compose.yml`).
3. **No broken script references:** `rg -n 'deploy/docker/(Dockerfile\.argo|docker-compose)' scripts/ Makefile .github/ deploy/smoke/` returns ZERO matches.
4. Diff is deletions + README repoints only (well under 500 LOC, mostly removals). No `cargo`/`vitest` build needed; the verifiers above ARE the test.

## On ship (same commit)
- Flip the WS5 deploy/Docker row in `docs/build-plan/FEATURE_MATRIX.md` to reflect the single canonical gateway stack (status badge + this commit SHA + `deploy/Dockerfile.gateway` + `deploy/docker-compose.yml`). Do NOT regenerate `docs/skycot/feature-matrix.html` (GITIGNORED per DEC-043 — `.md` row only).
- Note in the commit body that this removes the policy-less `deploy/docker/Dockerfile.argo` stack so no operator can deploy a gateway missing its truth-boundary policies + pitch corpus (closes the WS5 deploy-drift gap).

## Effort
S (deletions + two README repoints; no source, no migration, no test build — the `rg` verifiers are the gate).
