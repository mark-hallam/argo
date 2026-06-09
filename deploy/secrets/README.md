# deploy/secrets

Canonical production secret/config reference for the SKYCOT gateway.

- [`skycot-production.env.example`](skycot-production.env.example) — the single,
  code-grounded list of every secret + config a production deploy needs. Each var name
  is verified against the crate that reads it (citations inline). This is the superset;
  the partial templates (`deploy/.env.deploy.example`,
  `deploy/flyio/secrets.example.sh`) stay for their specific deploy paths.

**Never commit a filled-in copy.** `.gitignore` excludes `*.env` here.

## Where the values go

| Deploy path | How secrets are set |
|---|---|
| AWS (v1 operative) | AWS Secrets Manager — `infra/terraform/` creates the entries, operator populates via `aws secretsmanager put-secret-value` (runbook §6). |
| docker-compose / VM | Inject the filled `.env` directly into the gateway container. |

## Production boot prerequisite (read this)

The gateway runs in **production mode** in any real deploy (either
`ARGO_GATEWAY_PROFILE=production` or a non-loopback bind triggers it). Production mode
turns on a fail-closed at-rest-encryption gate. The gateway **refuses to boot** unless:

1. the image was built with `--features bundled-sqlcipher`, AND
2. `SKYCOT_DB_KEY_BASE64` is set to a real key.

The stock root `Dockerfile` build does **not** enable the SQLCipher feature — see the
runbook's "SQLCipher build" prerequisite before deploying.
