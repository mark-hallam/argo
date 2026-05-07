# Install

This quickstart takes you from a fresh machine to a working `argo`
binary in under five minutes.

## macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
argo setup
```

## Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.ps1 | iex
argo setup
```

## Verifying the install

```bash
argo doctor
```

This prints a JSON-shaped health report covering workspace state,
the embedded SQLite store, configured channels, and provider
credentials.

If `argo doctor` reports anything in the `WARN` or `ERROR` columns,
work through the [troubleshooting recipe](../howto/troubleshoot.md)
before continuing.

## Setting an LLM provider key

Pick one provider to start. The mock provider is fine for the
deterministic smoke test on the next page, but a real worker run
needs an actual key.

```bash
# Anthropic
argo cred set ANTHROPIC_API_KEY sk-ant-…

# OpenAI
argo cred set OPENAI_API_KEY sk-…
```

`argo cred` writes to `<workspace>/.skycot/credentials.toml` at mode
`0600`. For production deployments — systemd unit files, k8s
Secret, Vault, key rotation, what NOT to do — see
[`docs/skycot/provider_keys.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/provider_keys.md).

## What you have now

After running `argo setup`:

- `<workspace>/.skycot/` — artifact bundles, ledger files,
  credentials.
- `<workspace>/argo.toml` — workspace config.
- The embedded SQLite store wired through `argo doctor`.

## Next

Move on to [Your first worker](./first-worker.md) to see Argo
generate a vertical from a charter and run a single worker turn.
