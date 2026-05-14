# Data Residency

Use tenant-specific storage pools when a tenant's data must remain in
one legal region.

Configure each pinned tenant in `argo.toml`:

```toml
[storage.tenants.eu-customer]
db_url = "/var/lib/argo/eu-customer.db"
region = "eu-west-1"
```

Run migrations per tenant after deploying a new binary:

```bash
ARGO_DEPLOYMENT_REGION=eu-west-1 argo storage migrate-tenant eu-customer
argo storage list-tenants
argo storage health --json
```

The gateway compares `ARGO_DEPLOYMENT_REGION` with the requested
tenant's configured region. A mismatch returns HTTP 451 and records
`security.cross_region_blocked` in the audit chain.

Single-tenant deployments without `[storage.tenants]` keep using the
default `ARGO_HOME/sessions.db` path.
