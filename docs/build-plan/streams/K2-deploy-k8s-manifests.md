# K2 — Kubernetes manifests (Deployment + Service + Ingress + StatefulSet)

**Owner**: claude. **Depends on**: K1 image conventions (`argo:dev` / `argo-web:dev` local tags or OCI registry equivalents). Can land in parallel with K1; published OCI tags can be substituted later.
**Authored against**: origin/main `08c4ff04` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh K2-deploy-k8s-manifests`.

> SOURCE: DEPLOYMENT-PATH DIRECTIVE v3 §3 (Tier 5a). **Tier K (deployment recipe).** Second of three packaging recipes. Targets self-managed k8s clusters via plain manifests + kustomize overlays. NO Helm in v1 (operator request: keep deps minimal).

## Scope

NEW `deploy/k8s/` directory tree (≤ 700 LOC across ~13 files):

### Base manifests (`deploy/k8s/base/`)

- `argo-deployment.yaml` (NEW, ≤ 75 LOC) — Deployment, 1 replica default; container references `argo:dev` (operator overrides via kustomize); resource requests 250m CPU / 256Mi RAM; limits 1 CPU / 1Gi RAM; env from `argo-config` ConfigMap + `argo-secret` Secret; volume mount `/data` from `argo-data` PVC; liveness probe `/api/v1/health` 30s period; readiness probe `/api/v1/ready` 5s period.
- `argo-service.yaml` (NEW, ≤ 25 LOC) — ClusterIP service on port 8080.
- `argo-configmap.yaml` (NEW, ≤ 40 LOC) — non-secret env (ARGO_LOG_LEVEL, ARGO_TENANT_DEFAULT, NEXT_PUBLIC_GATEWAY_URL).
- `argo-secret.example.yaml` (NEW, ≤ 35 LOC) — example Secret manifest with `ARGO_GATEWAY_BEARER_TOKEN`, `DATABASE_URL`, optional `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`. Documented as EXAMPLE; operator copies + edits + applies separately (NEVER committed for real).
- `argo-pvc.yaml` (NEW, ≤ 20 LOC) — PersistentVolumeClaim for `argo-data` (10Gi default).
- `web-deployment.yaml` (NEW, ≤ 65 LOC) — Deployment, 2 replicas; container `argo-web:dev`; resource requests 100m/128Mi; limits 500m/512Mi; env `NEXT_PUBLIC_GATEWAY_URL=http://argo:8080`; probes hit `/` (web doesn't have /health route).
- `web-service.yaml` (NEW, ≤ 25 LOC) — ClusterIP service on port 3000.
- `ingress.yaml` (NEW, ≤ 60 LOC) — Ingress with TLS annotation for cert-manager + nginx; routes `/` → web service, `/api/` → argo service. Annotations: `cert-manager.io/cluster-issuer: letsencrypt-prod` (commented; operator opts in).
- `postgres-statefulset.yaml` (NEW, ≤ 80 LOC) — StatefulSet 1 replica, postgres:16-alpine, volumeClaimTemplate for `pg-data` (20Gi), env from `postgres-secret` (separate Secret operator manages), readiness probe `pg_isready`.
- `postgres-service.yaml` (NEW, ≤ 25 LOC) — Headless service (clusterIP: None) for StatefulSet stable network identity.
- `kustomization.yaml` (NEW, ≤ 25 LOC) — `resources:` lists all 10 above; commonLabels `app.kubernetes.io/part-of: skycot`.

### Overlays

- `deploy/k8s/overlays/dev/kustomization.yaml` (NEW, ≤ 30 LOC) — bases `../../base`; sets image tags to `argo:dev` / `argo-web:dev`; replicas 1; resource limits relaxed; no TLS on ingress.
- `deploy/k8s/overlays/prod/kustomization.yaml` (NEW, ≤ 40 LOC) — bases `../../base`; image tags set via `images:` directive (operator overrides per environment); replicas argo=1 + web=3; cert-manager annotation enabled; HorizontalPodAutoscaler enabled (CPU target 70%, min 1 max 4) — HPA itself is a sibling file under overlays/prod/.
- `deploy/k8s/overlays/prod/hpa-argo.yaml` (NEW, ≤ 30 LOC) — HPA for argo Deployment.
- `deploy/k8s/README.md` (NEW, ≤ 200 LOC) — operator runbook:
  - Prereqs: kubectl 1.28+; kustomize built-in (kubectl 1.14+) OR standalone; cert-manager if using TLS; ingress controller (nginx-ingress-controller recommended)
  - Quickstart dev (kind/minikube): `kind load docker-image argo:dev`; `kubectl apply -k deploy/k8s/overlays/dev`
  - Prod: edit `overlays/prod/kustomization.yaml` images, create secrets, `kubectl apply -k deploy/k8s/overlays/prod`
  - Probes + scaling, backup recipe (`kubectl exec` into postgres pod + pg_dump), upgrade pattern, troubleshooting.

This is NOT in scope: Helm chart packaging (separate brief; not requested for v1); operator-pattern CRDs (overkill for v1); service mesh (Istio / Linkerd — out of scope; operator opts in); pod-security-policy / NetworkPolicy (operator-environment-specific; v2 brief if needed); cluster-autoscaler integration (cluster-level concern); externalDNS / cluster-issuer auto-config (operator manages these out-of-band); ArgoCD / Flux GitOps integration (operator can wire kustomize bases into their existing GitOps surface); managed-service variants (RDS for postgres, etc. — operator points DATABASE_URL externally).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `deploy/k8s/base/*.yaml` (NEW, see scope)
- `deploy/k8s/overlays/dev/kustomization.yaml` (NEW)
- `deploy/k8s/overlays/prod/kustomization.yaml` (NEW)
- `deploy/k8s/overlays/prod/hpa-argo.yaml` (NEW)
- `deploy/k8s/README.md` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- `deploy/docker/**` (K1's scope)
- `crates/**` (no Rust changes)
- `.github/workflows/**` (K5's scope)
- Any vertical manifest

## Acceptance criteria

- [ ] `kubectl apply -k deploy/k8s/overlays/dev --dry-run=client` succeeds (no schema errors)
- [ ] `kubectl apply -k deploy/k8s/overlays/dev` on kind/minikube reaches Ready for all pods in < 90s
- [ ] Liveness + readiness probes hit real endpoints (not just `/`)
- [ ] `kubectl get all -l app.kubernetes.io/part-of=skycot` returns all resources
- [ ] Ingress with `--watch` resolves; curl through ingress IP returns SPA HTML
- [ ] Secret example is NEVER applied directly; README directs operator to copy + edit + apply separately
- [ ] HPA stub validates (kubectl apply --dry-run=client) even if not deployed in dev overlay
- [ ] Image references resolve via kustomize image transformer (operator can `images: [{name: argo, newTag: v1.2.3}]`)
- [ ] All YAML passes `kubectl explain` schema validation for k8s 1.28+
- [ ] No new file > 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-K2-deploy-k8s-manifests`
- PR title: `deploy: k8s manifests (Deployment + Service + Ingress + kustomize overlays) (K2)`

## Notes for the implementer

- `kustomize` is the v1 packaging primitive — built into kubectl since 1.14, no extra install required. Don't use Helm.
- Image references in base manifests use `argo:dev` / `argo-web:dev` as placeholders; kustomize overlays set the real tags via `images:` directive. This decouples the base from any specific registry.
- The example Secret manifest is the trickiest — operators have shot themselves in the foot before by committing actual secrets. The example file MUST have placeholder values (`<base64-encoded-token>`) and the README MUST explain the operator creates the real secret via `kubectl create secret generic argo-secret --from-literal=...`.
- For PostgreSQL: a StatefulSet with a single replica is the v1 path. HA postgres is operator-managed (point DATABASE_URL at managed RDS / AlloyDB / similar). Don't try to implement HA in v1.
- The Ingress assumes nginx-ingress-controller is installed and the cluster has a default IngressClass. Operators on Traefik / HAProxy / Istio gateway will need to substitute — document this in the README's prereqs.
- Resource requests/limits are dev-baseline; operators tune via overlays. Don't over-spec defaults.
- HPA stub in prod overlay: target 70% CPU is the autoscaler default; `min: 1 max: 4`. Operators tune. The HPA file is included so the YAML structure is right; deployment is opt-in via the overlay.
- The `web` Deployment has 2 replicas in prod by default (vs argo's 1). The argo binary is currently single-instance per-tenant (the existing worker pool is internal to one binary); horizontal scaling is a future brief. The web SPA is stateless and trivially horizontally scaled.
- DO NOT introduce a separate migrator Job; argo's binary runs migrations on first connect to postgres. Add a note in README that the first pod start takes longer (15-30s) on a fresh DB.
- Verify k8s API versions at claim time. `apps/v1` for Deployment / StatefulSet; `v1` for Service / ConfigMap / Secret / PVC; `networking.k8s.io/v1` for Ingress; `autoscaling/v2` for HPA (NOT v1, which is deprecated).
- The K5 smoke workflow won't directly drive k8s; it uses K1 docker-compose. But k8s manifests can be smoke-tested via `kubectl apply --dry-run=client -k overlays/dev` in a CI step — add that to README as an optional CI hook.
