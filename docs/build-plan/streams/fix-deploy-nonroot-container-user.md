# fix-deploy-nonroot-container-user

**Owner**: claude.
**Authored against**: origin/main `306d240523b5643b3863fd9d3494e2e3be76b784` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-deploy-nonroot-container-user`.
**Source**: AWS deploy-readiness audit `docs/skycot/audit/2026-06-14-aws-deploy-readiness.md` — HIGH finding under "Containers" (punch-list line 23; HIGH table row "All images run as root").

🔴 **P2 (HIGH) — every runtime container image runs as root (UID 0). Add a non-root `USER` (uid 10001) to all three runtime Dockerfiles plus a k8s `securityContext`. deploy/security-critical → conductor adversarial review before merge; do NOT self-merge.**

## Problem statement (file:line evidence, verified this turn on `306d2405`)

All three runtime container images run their process as **root (UID 0)** — there is no `USER` directive anywhere, and the k8s Deployment has no `securityContext`. A container compromise (the gateway parses untrusted Stripe/Discord/SMS webhooks and runs computer-use) therefore starts as root inside the container, which is a needless escalation surface for a paying-user launch.

Verified this turn (`grep -rn '^USER ' Dockerfile deploy/Dockerfile.gateway deploy/Dockerfile.web` → **0 hits**; `grep -rn 'securityContext\|runAsNonRoot\|runAsUser' deploy/k8s/` → **0 hits**; `grep -rn '10001\|appuser\|nonroot' deploy/ Dockerfile` → **0 hits**):

- `deploy/Dockerfile.gateway:38-60` — runtime stage `FROM debian:bookworm-slim`; ends `CMD ["argo", "gateway", "start", ...]` with **no `USER`**. Binds `127.0.0.1:7400` (`:58 EXPOSE 7400`); writes its store under `ARGO_HOME=/data` (`:56`, `VOLUME ["/data"]` at `:57`).
- `deploy/Dockerfile.web:41-52` — runtime stage `FROM node:20-bookworm-slim`; ends `CMD ["node", "server.js"]` with **no `USER`**. Binds `127.0.0.1:3000` (`:44 HOSTNAME=127.0.0.1 PORT=3000`, `:51 EXPOSE 3000`). Static-only Next standalone output; does not write a persistent volume.
- `Dockerfile:48-86` (root / ECS image) — runtime stage `FROM debian:bookworm-slim`; ends `ENTRYPOINT ["/usr/local/bin/argo"]` + `CMD ["gateway", "start", "--bind", "0.0.0.0", ...]` with **no `USER`**. Binds `0.0.0.0:8080` (`:84 EXPOSE 8080`); writes its store under `ARGO_HOME=/data` (`:78`).
- `deploy/k8s/base/argo-deployment.yaml:21-44` — `spec.template.spec` has **no pod `securityContext` and no container `securityContext`**; the container mounts the `/data` PVC at `:61-63`.

Load-bearing facts for the fix (so a non-root switch does not break boot):
- **All exposed ports are > 1024** (7400 / 8080 / 3000), so a non-root user can bind them with **no** `CAP_NET_BIND_SERVICE` capability. No privileged-port handling is needed.
- **The two gateway images write `/data` (`ARGO_HOME`)** — the SQLCipher store + `~/.argo` runtime state live there. A non-root UID must **own or be able to write** `/data`, including on a fresh empty volume (EFS/PVC mounts come up root-owned by default). This is the one thing most likely to break; the fix must handle it (`chown`/`mkdir` the dir to the non-root uid at build time, and/or rely on the k8s `fsGroup` for the PVC).
- The web image writes no persistent dir, so it is the simplest case.

## May-touch (explicit)

- `deploy/Dockerfile.gateway` — add a non-root user + `USER 10001` to the runtime stage; ensure `/data` (and `ARGO_HOME`) is writable by that uid.
- `deploy/Dockerfile.web` — add a non-root user + `USER 10001` to the runtime stage.
- `Dockerfile` (repo root / ECS image) — add a non-root user + `USER 10001` to the runtime stage; ensure `/data` is writable by that uid.
- `deploy/k8s/base/argo-deployment.yaml` — add pod + container `securityContext` (`runAsNonRoot: true`, `runAsUser: 10001`, `runAsGroup: 10001`, `allowPrivilegeEscalation: false`, and `fsGroup: 10001` at pod level so the `/data` PVC is group-writable).
- `docs/build-plan/FEATURE_MATRIX.md` — update the relevant container/deploy-hardening row in-band, same commit (status badge + commit SHA + the touched files). **The `FEATURE_MATRIX.md` markdown IS the source-of-truth ledger; do NOT commit the generated `docs/skycot/feature-matrix.html`.**
- (docs, optional) `docs/skycot/operator/AWS_DEPLOY_RUNBOOK.md` and/or `docs/skycot/operator/PRODUCTION-RUNBOOK.md` — one note that the images now run non-root (uid 10001) and that the EFS/PVC mount must be group-writable to 10001.

**Out of scope** (other audit findings, do NOT fold in): the `smtp_password` secret / email-wiring blockers (B1), the `front_with_cloudflare` default (B2), the offsite-backup module, CI re-enable, the k8s secret-set / postgres-scaffold / image-pin cleanups (those are a separate MEDIUM k8s brief — this brief only adds the `securityContext`). Keep this PR to the non-root user change.

## Parallel-surface declaration

**No new public Rust types, structs, enums, or `type` aliases.** This change is confined to container build files (`Dockerfile`, `deploy/Dockerfile.gateway`, `deploy/Dockerfile.web`), one k8s YAML manifest, and the ledger/docs. No `crates/` source is touched, so the grep-before-write rule for new `pub` surfaces does not apply. Verified there is no existing non-root user convention to collide with: `grep -rn '10001\|appuser\|nonroot' deploy/ Dockerfile` → **0 hits** this turn, so `uid 10001` (named e.g. `app`) is a new, non-colliding choice.

## Fix recipe

1. **`deploy/Dockerfile.web`** (simplest, no volume): in the runtime stage, after the `COPY` lines, create a non-root user and switch to it, e.g.
   `RUN groupadd -g 10001 app && useradd -u 10001 -g 10001 -m -s /usr/sbin/nologin app` then `USER 10001`. Confirm `node server.js` still binds 3000.
2. **`deploy/Dockerfile.gateway`** and **`Dockerfile`** (both write `/data`): create the same uid/gid, **`mkdir -p /data && chown 10001:10001 /data`** in the runtime stage before `USER 10001`, so a fresh empty `/data` is writable by the non-root process. Keep `VOLUME ["/data"]` / `ARGO_HOME=/data` as-is. Put `USER 10001` AFTER the `COPY`/`chown` lines so the copies still land as root-readable.
3. **`deploy/k8s/base/argo-deployment.yaml`**: add pod-level `securityContext: { runAsNonRoot: true, runAsUser: 10001, runAsGroup: 10001, fsGroup: 10001 }` and a container-level `securityContext: { allowPrivilegeEscalation: false }`. The `fsGroup` makes the mounted `/data` PVC group-writable to 10001 (the in-image `chown` does not cover a PVC mount). Do not add a read-only root filesystem in this brief (the gateway writes `/data` and may write temp paths) — leave that as a follow-up.
4. Keep every file **< 500 LOC** and any added shell snippet trivial (no functions; the < 80-LOC/fn rule is not at risk here).

## Acceptance criteria (CHECKABLE verifier — NO THEATER)

- [ ] **Non-root proof per image** — for each of the three images, build it and run `id -u`; the command returns a **non-zero** uid (10001), not 0:
  - `docker build -f Dockerfile -t argo-prod-nonroot . && docker run --rm --entrypoint id argo-prod-nonroot -u` → prints `10001` (NOT `0`).
  - `docker build -f deploy/Dockerfile.gateway -t argo-gw-nonroot . && docker run --rm --entrypoint id argo-gw-nonroot -u` → `10001`.
  - `docker build -f deploy/Dockerfile.web -t argo-web-nonroot . && docker run --rm --entrypoint id argo-web-nonroot -u` → `10001`.
  - Paste all three outputs in the PR body. (`-u` returning a non-zero number is the binary go/no-go.)
- [ ] **Gateway still writes `/data` + binds its port as non-root** — the production-image boot smoke still passes (it boots the ROOT `Dockerfile` as a server, opens the SQLCipher store under `/data`, waits for `/health/live` 200, then probes `POST /api/v1/pitch/ask/public` for `200` + `source=canonical`):
  - `bash deploy/smoke/production-image-boot-smoke.sh` → exits 0, ends `PASS: production image boots and serves...`. (A `/data`-write or port-bind failure under the new uid makes the gateway die before binding, which this script fails loudly on.)
- [ ] **Compose smoke still green** (exercises `deploy/Dockerfile.gateway` + `deploy/Dockerfile.web` together behind nginx):
  - `bash deploy/smoke/run.sh` → exits 0 (the 4/4 green path recorded in `deploy/smoke/LAST-PASS.md`).
- [ ] **k8s manifest is valid + asserts non-root** — `runAsNonRoot: true`, `runAsUser: 10001`, `allowPrivilegeEscalation: false`, and `fsGroup: 10001` are all present in `deploy/k8s/base/argo-deployment.yaml`; if `kubectl` is available, `kubectl apply --dry-run=client -f deploy/k8s/base/argo-deployment.yaml` succeeds. (If `kubectl` is unavailable in the worker env, a `grep` showing all four keys present is the fallback evidence — state which in the PR body.)
- [ ] **Ledger updated in-band** — the relevant `docs/build-plan/FEATURE_MATRIX.md` row is updated in the SAME commit (status + SHA + touched files); the generated `feature-matrix.html` is NOT committed.

## Review gate

**deploy/security-critical → conductor adversarial review before merge; do NOT self-merge.** This changes the runtime container privilege model and the EFS/PVC write path; a wrong `chown`/`fsGroup` could crash-loop the gateway on a fresh volume (the exact `exit-78` failure class the audit's B3 flags). The conductor re-verifies the three `id -u` outputs + both smoke results + the k8s keys as VERIFIED (not relayed) before merge.

## Effort

~30-60 LOC across 3 Dockerfiles + 1 k8s YAML + the ledger row. ~1-2 hours including the two smoke runs.
