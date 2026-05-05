# Provider keys + secrets flow

How to wire provider API keys into SKYCOT for local dev,
testing, and production deployments. This is the doc you
read before going live with a paying customer.

## What needs a key

SKYCOT calls an LLM provider on every multi-step worker turn.
The shipped verticals default to:

```toml
[provider]
default  = "anthropic"
fallback = "openai"
```

So you need at least one of `ANTHROPIC_API_KEY` or
`OPENAI_API_KEY`. Mock provider (`--provider mock`) needs
nothing — it produces deterministic synthetic output for
smoke tests.

## Local development — the `argo setup` flow

`argo setup` is the canonical first-time onboarding command:

```bash
argo setup
```

It detects which provider env-vars are already exported and, if
none are set, walks the operator through picking one and adding
the right `export …` line to their shell rc.

To verify a configured environment without going through the
walkthrough (handy for CI or a launch checklist):

```bash
argo setup --check
```

Exits 0 when at least one provider key is loadable from the
process env, non-zero with a clear FAIL message + resolution-
order summary on stderr otherwise.

## Order of resolution

When the worker (or any `argo` command that calls a provider)
needs a key, it reads the **process environment**:
`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY` etc.
exported in the shell (or unit file / pod env) that invoked
`argo`.

The vertical's `[provider]` declaration only specifies which
provider to USE — never the key itself.

A future v1.x release will add a `~/.config/argo/secrets.toml`
fallback so operators can keep keys out of shell history; track
via the SKYCOT roadmap. Until then, env is the single source.

If the key is missing, the worker fails the next provider call
with a transport-error stop reason. See
[`failure_modes.md`](failure_modes.md) § Provider failures
for recovery.

## Production patterns

### Pattern A — env vars from a process supervisor

Easiest for single-host deployments. Set keys in the
supervisor's unit file:

```ini
# systemd service file
[Service]
Environment="ANTHROPIC_API_KEY=<key>"
Environment="OPENAI_API_KEY=<fallback-key>"
ExecStart=/usr/local/bin/argo work serve --vertical gtm-engineer
```

Or with envFiles:

```ini
[Service]
EnvironmentFile=/etc/argo/secrets.env
ExecStart=/usr/local/bin/argo work serve --vertical gtm-engineer
```

Where `/etc/argo/secrets.env` is `0600`-perm with:

```env
ANTHROPIC_API_KEY=sk-ant-…
OPENAI_API_KEY=sk-…
```

### Pattern B — Kubernetes Secret + envFrom

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: argo-providers
type: Opaque
stringData:
  ANTHROPIC_API_KEY: "sk-ant-…"
  OPENAI_API_KEY: "sk-…"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: skycot-worker
spec:
  replicas: 1                      # singleton, see failure_modes § Concurrent workers
  template:
    spec:
      containers:
      - name: worker
        image: <your-argo-image>
        envFrom:
          - secretRef:
              name: argo-providers
        command: ["/usr/local/bin/argo"]
        args: ["work", "serve", "--vertical", "gtm-engineer"]
```

### Pattern C — runtime fetch from a secret manager

For Vault / 1Password / AWS Secrets Manager / GCP Secret
Manager, fetch into env at process start:

```bash
# wrapper script: scripts/start-worker.sh
#!/usr/bin/env bash
set -euo pipefail
export ANTHROPIC_API_KEY="$(vault kv get -field=key secret/skycot/anthropic)"
export OPENAI_API_KEY="$(vault kv get -field=key secret/skycot/openai)"
exec argo work serve --vertical gtm-engineer "$@"
```

The point is: **SKYCOT itself doesn't talk to your secret
manager**. Wrap the binary; let your existing secrets
infrastructure handle delivery.

## What you must NOT do

- **Don't commit `.env` files or systemd `EnvironmentFile=`
  paths to the repo.** If your tooling writes one alongside the
  repo, add that path to your `.gitignore`.
- **Don't put keys in `vertical.toml`.** The whole verticals
  tree is readable by the worker AND the gateway AND any
  agent dispatched against it. Keys belong outside the
  vertical.
- **Don't paste keys into prompts.** The chat history is
  persisted in the SQLite store + dumped to artifact bundles.
  Anything in a prompt is in the audit trail forever.
- **Don't share env vars across operators.** Each operator
  should have their own keys, not least so audit + spend
  tracking is per-operator.

## Verifying a setup

```bash
argo setup --check          # confirm keys are loadable
argo doctor                 # broader health check (incl. provider availability)
argo worker doctor          # SKYCOT-specific health
```

For a non-mock end-to-end smoke against a real provider, queue
+ run a tiny job:

```bash
argo work add --vertical gtm-engineer --prompt "smoke" --json
argo work serve --vertical gtm-engineer --once \
  --provider anthropic --model claude-sonnet-4-5 \
  --max-steps 1 --json
```

If the provider call is wired correctly, the
`completed_jobs.stop_reason` will NOT be `need_info` (which
indicates a transport error). Inspect the artifact bundle's
`transcript.md` to confirm the provider actually responded.

## Provider fallback

When the primary provider 503s or rate-limits, the worker
falls back to the vertical's declared `fallback` provider on
its next step. Configure both:

```toml
[provider]
default   = "anthropic"
fallback  = "openai"
cost_tier = "balanced"        # primary's tier
```

Make sure both providers' keys are loadable by the resolution
order above. If the fallback key is missing, you'll see a
clear error in the artifact bundle and the worker stops with
`need_info`.

## Key rotation

The OS doesn't cache keys past process start. To rotate:

1. Update the key in your secrets store / env.
2. Restart the worker (`systemctl restart skycot-worker` /
   `kubectl rollout restart deployment/skycot-worker`).
3. The next worker turn picks up the new key.

In-flight worker steps will continue to use the OLD key
until the step finishes. Plan rotation accordingly:

- **Soft rotation**: deploy the new key, wait for the
  current job to finish, then restart. Zero downtime, no
  in-flight failures.
- **Hard rotation** (revoking compromised keys): restart
  immediately. In-flight step fails with a transport error;
  see [`failure_modes.md`](failure_modes.md) § Provider
  failures for recovery.

## Spend tracking

Every provider call writes a `spend_ledger` row. The
gateway's `/metrics` endpoint exposes
`argo_spend_usd_micros_total` so monitoring can alert on
unexpected spend curves. See `argo costs` for the operator-
facing query surface.

For per-tenant spend caps (auto-throttle at $X/hour), the
v1.0 budget primitive is on the roadmap (#75x); today the
operator monitors via `/metrics` + alerts.

## Production launch checklist for keys

- [ ] Both `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` (or your
      chosen pair) are loadable from the worker's process env.
- [ ] Keys are NOT in any committed file (`grep -rn "sk-ant-"
      .` returns nothing).
- [ ] `argo setup --check` passes against the production env.
- [ ] At least one non-mock smoke job ran end-to-end and the
      stop_reason was not `need_info`.
- [ ] Key rotation procedure tested at least once before
      first paying customer.
- [ ] `/metrics` is being scraped + spend has an alert
      threshold.
