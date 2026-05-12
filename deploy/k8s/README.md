# Skycot — Kubernetes deployment recipe

Plain Kubernetes manifests + kustomize overlays for the `argo` gateway,
the `argo-web` SPA, and a single-instance `postgres` backend on a
self-managed cluster. **No Helm.**

Pairs with `deploy/docker/` (K1): docker-compose is the single-machine
path; this directory is the cluster path. Both consume the same
`argo:dev` / `argo-web:dev` image conventions.

## Prerequisites

- `kubectl` 1.28+ (built-in `kustomize` is fine; standalone `kustomize`
  v5.x also works).
- A reachable Kubernetes cluster (kind, minikube, k3d, EKS, GKE, AKS,
  bare metal).
- An ingress controller. Manifests assume **nginx-ingress-controller**
  with `ingressClassName: nginx`. Operators on Traefik / HAProxy /
  Istio gateway substitute `ingressClassName` and the controller-
  specific annotations in `base/ingress.yaml`.
- (Prod only) [cert-manager](https://cert-manager.io/) + a
  `ClusterIssuer` for automated TLS.
- Container images. Either build locally per `deploy/docker/` and load
  into the cluster (`kind load docker-image …`) or push to a registry
  and override `images:` in the relevant overlay.

## Layout

```
deploy/k8s/
├── base/                          # canonical manifests
│   ├── argo-{configmap,deployment,pvc,service}.yaml
│   ├── argo-secret.example.yaml   # NOT in kustomization; reference only
│   ├── ingress.yaml
│   ├── postgres-{service,statefulset}.yaml
│   ├── web-{deployment,service}.yaml
│   └── kustomization.yaml
└── overlays/{dev,prod}/kustomization.yaml   # + prod/hpa-argo.yaml
```

## Quickstart — local dev (kind / minikube / k3d)

```bash
# 1. Build images per deploy/docker/README.md, then load into the cluster.
docker build -f deploy/docker/Dockerfile.argo -t argo:dev .
docker build -f deploy/docker/Dockerfile.web  -t argo-web:dev .
kind load docker-image argo:dev argo-web:dev   # or minikube/k3d image load

# 2. Create the two Secrets the manifests reference.
kubectl create namespace skycot-dev
kubectl -n skycot-dev create secret generic argo-secret \
  --from-literal=ARGO_GATEWAY_BEARER_TOKEN="$(openssl rand -hex 32)" \
  --from-literal=DATABASE_URL="postgres://argo:devpw@postgres:5432/argo" \
  --from-literal=ANTHROPIC_API_KEY="" \
  --from-literal=OPENAI_API_KEY=""
kubectl -n skycot-dev create secret generic postgres-secret \
  --from-literal=POSTGRES_USER=argo \
  --from-literal=POSTGRES_PASSWORD=devpw

# 3. Apply the dev overlay; pods reach Ready in <90s on a 4-CPU host.
kubectl apply -k deploy/k8s/overlays/dev
kubectl -n skycot-dev get pods -w
```

The first `argo` pod start takes 15-30s longer than restarts because
the binary applies database migrations on its first connect to a fresh
postgres — there is no separate migrator Job.

Smoke-test:

```bash
kubectl -n skycot-dev port-forward svc/argo 8080:8080 &
curl -s http://localhost:8080/health/live
curl -s http://localhost:8080/health/ready
kubectl -n skycot-dev port-forward svc/web  3000:3000 &
curl -s http://localhost:3000/ | grep -q '<html'
```

Tear down:

```bash
kubectl delete -k deploy/k8s/overlays/dev
kubectl -n skycot-dev delete pvc -l app.kubernetes.io/part-of=skycot
kubectl delete namespace skycot-dev
```

## Production deploy

1. Edit `overlays/prod/kustomization.yaml`:
   - Set `images:` `newName` + `newTag` to your published refs (e.g.
     `ghcr.io/your-org/argo:v1.2.3`).
   - Replace `argo.example.com` in the Ingress patch with the real
     hostname.
   - Set `cert-manager.io/cluster-issuer` to your ClusterIssuer's
     name (or remove the patch if TLS terminates upstream).

2. Create production Secrets out-of-band. **Never commit
   `argo-secret.example.yaml` with real values.** Prefer Sealed Secrets
   / External Secrets / Vault. Plain `kubectl create secret` is an
   acceptable bootstrap (same shape as the dev step above; just stronger
   passwords + real provider keys).

3. `kubectl apply -k deploy/k8s/overlays/prod`.

4. `kubectl -n skycot get hpa argo` — metrics column reads
   `<unknown>/70%` until metrics-server scrapes enough samples. If
   metrics-server isn't installed, install it or remove `hpa-argo.yaml`
   from the overlay's `resources:`.

## Secrets

`argo-secret.example.yaml` exists **only as documentation**. It is
deliberately excluded from `base/kustomization.yaml` so a stray
`kubectl apply -k` never installs the placeholder values. The base
manifests reference `argo-secret` and `postgres-secret` by name and
rely on the operator to create the real objects.

## Probes and scaling

- `argo` liveness probe → `/health/live` (200 if the process is alive).
  Readiness probe → `/health/ready` (200 only when the store responds —
  gates traffic until migrations finish).
- `web` probes hit `/` because the Next.js standalone server has no
  dedicated health route.
- `argo` defaults to `replicas: 1` (single-instance per-tenant; the
  worker pool is internal to one binary). Horizontal scaling for
  `argo` is a future brief; the prod HPA is wired in as a stub so the
  structure is in place when the runtime supports it.
- `web` is stateless and trivially horizontal — `replicas: 3` in prod,
  `1` in dev.

## Backup recipe

For HA postgres, point `DATABASE_URL` at a managed service (RDS /
AlloyDB / Cloud SQL) and remove the `postgres-*` files from the base
kustomization. For the in-cluster StatefulSet, take logical backups:

```bash
kubectl -n skycot exec -it postgres-0 -- \
  pg_dump -U argo -d argo --no-owner --format=custom \
  > argo-$(date +%Y%m%d-%H%M%S).pgdump
```

Restore via `pg_restore --clean --if-exists` over `kubectl exec -i`.
PVC snapshots (CSI VolumeSnapshot) are the operator's responsibility.

## Upgrade / rollback

Bump `images.newTag` in the prod overlay, then:

```bash
kubectl apply -k deploy/k8s/overlays/prod
kubectl -n skycot rollout status deployment/argo
kubectl -n skycot rollout status deployment/web
# Rollback:
kubectl -n skycot rollout undo deployment/argo
```

The `argo` Deployment uses `strategy: Recreate` (single replica + a
`ReadWriteOnce` PVC means a rolling upgrade would deadlock on the
volume). Expect ~15s of downtime per `argo` upgrade; web rolls without
downtime.

## Troubleshooting

- **`argo` crashlooping with `connection refused`** — postgres still
  starting. `argo` retries on its own; k8s has no cross-resource
  `depends_on`.
- **`Pending` PVC** — no default StorageClass. Create one or patch the
  PVC manifests with `storageClassName`.
- **Ingress 404** — `kubectl describe ingress argo`; confirm
  `ingressClassName` matches your installed controller.
- **HPA `<unknown>` metrics** — install `metrics-server` or remove the
  HPA from the overlay.
- **Secret keys missing** — pods fail with `couldn't find key X in
  Secret …`. The `kubectl create secret` step must list every key the
  Deployment references (see `argo-secret.example.yaml`).

## CI / dry-run validation

```bash
kubectl apply -k deploy/k8s/overlays/dev  --dry-run=client -o yaml > /dev/null
kubectl apply -k deploy/k8s/overlays/prod --dry-run=client -o yaml > /dev/null
```

Both exit 0 with no schema errors. K5's smoke workflow drives the
docker-compose stack rather than k8s; the dry-run check above is the
lightweight gate for manifest validity.

## Out of scope

Helm chart, ArgoCD/Flux wiring, NetworkPolicy/PodSecurity, service
mesh, managed-service variants. Operator opt-in for any of the above.
