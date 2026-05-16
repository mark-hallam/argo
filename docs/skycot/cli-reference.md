# CLI reference

| Command | Purpose |
| --- | --- |
| `argo config show [--key <path>] [--json] [--source] [--show-secrets]` | Print the effective merged configuration; redacts secret-looking fields unless `--show-secrets` is paired with `ARGO_SHOW_SECRETS=1`. |
| `argo doctor [--skip-network] [--json]` | Runs workspace, database, gateway, DNS, TLS, and provider-auth checks. Set `ARGO_DOCTOR_SKIP_NETWORK=1` or pass `--skip-network` for offline CI and airgapped environments. |
