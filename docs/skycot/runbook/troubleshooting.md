# Operator troubleshooting

Five operator-reported failures that surface as opaque-looking
errors but are 30-second fixes once you know what to look at.
For the established failure-mode table (executor stubs,
manifest validation, approval-gate errors) see
[Observability §9.6](observability.md#96-common-failure-modes);
for the gateway-startup exit-78 table see
[Observability §9.7](observability.md#97-startup-health-check-exit-78).

## Vertical manifest schema_version mismatch

**Symptom** — `argo vertical load` (and anything that loads a
vertical, including `argo run` and `argo gateway start` against
a tenant that references the vertical) refuses with:

```text
vertical manifest schema_version '99' is not supported by this argo build (expected '1')
```

The literal version numbers will differ. The `expected '...'`
suffix is the version this binary speaks.

**Cause** — `vertical.toml`'s top-level `schema_version` key
doesn't match the loader's expected version
([`crates/argo-vertical/src/manifest.rs`](../../../crates/argo-vertical/src/manifest.rs)).
Either the manifest was hand-edited against a future schema, or
this binary is behind the manifest's authoring binary. Argo
versions are forward-only; there is no compatibility shim across
schema versions.

**Fix**

1. Read the expected version from the error message (the value
   after `expected`).
2. Open the offending `vertical.toml`. The path is in the
   command's stderr immediately above the error.
3. Edit the `schema_version = "..."` line to match.
4. Re-run the loader. If the manifest genuinely needs the
   newer schema, upgrade the binary instead: `cargo install
   --path crates/argo-cli` from a fresh checkout.

**Related** — [migrations](../RUNBOOK.md#migrations) (forward-only
schema policy applies to both DB migrations and manifest schemas).

## Gateway port already bound

**Symptom** — `argo gateway start` exits non-zero shortly after
the startup health check passes:

```text
argo gateway: startup_check ok in 12 ms
Error: bind 127.0.0.1:7400: Address already in use (os error 48)
```

A sibling form fires when a previous `argo gateway` is still
running:

```text
Error: gateway already running (pid 47213, see /Users/you/.argo/gateway.pid)
```

**Cause** — Something else is listening on the gateway's
target port (default `7400`; configurable via `--port`). Most
common offenders:

- A previous `argo gateway` that survived a crash without
  cleaning up.
- A second `argo` tab started in the same workspace.
- An unrelated dev server squatting on `7400` (Node/Vite,
  another HTTP daemon).

**Fix**

1. Identify the squatter:
   ```bash
   lsof -nP -iTCP:7400 -sTCP:LISTEN
   ```
2. If it's a stale `argo gateway`, stop it cleanly:
   ```bash
   argo gateway stop
   ```
   This sends `SIGTERM` to the pid in `~/.argo/gateway.pid`.
   If `argo gateway stop` complains about a stale PID file, it
   has already cleaned it up — re-run `argo gateway start`.
3. If it's something else, either kill the squatter or pick a
   free port:
   ```bash
   argo gateway start --port 7500
   ```
4. Verify the new boot:
   ```bash
   argo gateway probe live --port 7500
   ```
   Expected: `ok`, exit 0.

**Related** —
[Production deploy §12](production-deploy.md#12-production-deploy)
(reverse-proxy + bind-host setup),
[Observability §9.7](observability.md#97-startup-health-check-exit-78)
(distinguishing exit 78 from bind-failure exits).

## Missing provider API key

**Symptom** — `argo gateway start` refuses to bind with the
exit-78 banner:

```text
argo gateway: STARTUP CHECK FAILED (startup check: no provider API key set (need ANTHROPIC_API_KEY or OPENAI_API_KEY)); refusing to bind
```

When a worker is already running and the env var is unset
mid-session, the next model dispatch fails with:

```text
Error: missing API key for env ANTHROPIC_API_KEY
```

(The env-var name in the second message varies by provider —
`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc. — matching the
provider manifest's `auth.api_key_env` field.)

**Cause** — The runtime can't find a credential for the
configured model provider. Most common:

- The shell that launched `argo` doesn't have the key exported
  (`.zshenv` vs `.zshrc` — non-interactive subshells skip
  `.zshrc`; see
  [SWARM_PROTOCOL §One-time environment setup](../../build-plan/SWARM_PROTOCOL.md#one-time-environment-setup)).
- The key was rotated and the new value hasn't been re-exported.
- The credentials store is encrypted but its master key is
  missing — see [Key rotation](../RUNBOOK.md#key-rotation).

**Fix**

1. Confirm the key isn't already set:
   ```bash
   echo "${ANTHROPIC_API_KEY:-unset}"
   ```
2. Export it (replace with your real key):
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```
   For persistence across shell sessions, add to `~/.zshenv`
   (macOS) or `~/.profile` (Linux). Use
   [direnv](https://direnv.net) for a per-repo `.envrc` if you
   want the key scoped to one project.
3. Re-run the failing command. The gateway picks up env vars
   at startup time — restart `argo gateway` after export.

**Related** —
[Observability §9.7](observability.md#97-startup-health-check-exit-78)
(other exit-78 failure modes),
[Key rotation](../RUNBOOK.md#key-rotation) (rotating the live
key without downtime).

## Unknown channel in a send-message tool call

**Symptom** — A worker action that calls the `send_message`
tool returns an error envelope containing:

```text
unknown channel 'slack-dm': supported = email | slack | sms | push | discord | whatsapp
```

The handoff notifier surfaces a related form when an outbound
address scheme has no channel wired:

```text
Error: no channel wired for scheme 'sms' (addr sms:+15551234); waiting on follow-up brief
```

**Cause** — The action requested a channel id (`slack-dm`) or
address scheme (`sms:`) that the runtime doesn't recognise or
hasn't been bound to a connector. The full supported set is
the `SUPPORTED` constant in
[`crates/argo-tool/src/tools/send_message.rs`](../../../crates/argo-tool/src/tools/send_message.rs)
plus whatever bindings live in your `.skycot/notify.toml`
`[notify_channels]` block.

**Fix**

1. Check the action / charter for the bad channel id:
   ```bash
   argo worker actions show <action-id>
   ```
   Look at the `arguments.channel` field. Replace with one of
   the listed supported channels (e.g. `slack` instead of
   `slack-dm`).
2. If the channel id is correct but the connector isn't wired,
   open `.skycot/notify.toml` and add a binding for it. See
   [Workers and approvals §5](workers.md#5-configure-approvals).
3. For address-scheme errors (`no channel wired for scheme`),
   either:
   - rewrite the address to use a supported scheme
     (`mailto:` for email, `slack:` for Slack DMs), or
   - wire the missing scheme by adding the corresponding
     adapter to `[gateway].adapters` in `argo.toml`.
4. Re-dispatch the action:
   ```bash
   argo worker actions retry <action-id>
   ```

**Related** —
[Workers and approvals §5](workers.md#5-configure-approvals)
(channel-binding setup),
[Approvals JSON API](../RUNBOOK.md#approvals-json-api) (re-driving
stuck actions through the JSON surface).

## Unknown executor kind on vertical load

**Symptom** — Loading a vertical (`argo vertical validate`,
`argo run`, or anything that exercises the manifest validator)
fails with a per-action error like:

```text
actions.draft_reply.executor.kind: unknown executor kind 'nonsense' — valid: human | mcp | computer_use | computer_use_macos | computer-use-macos | computer_use_linux | computer-use-linux | computer_use_windows | computer-use-windows | api | browser | filesystem | code_execution | document_creation | document_csv | document-csv
```

The `actions.<id>.executor.kind` prefix is the TOML path of
the bad field, so you can locate it by reading the manifest
left-to-right.

**Cause** — The vertical's `vertical.toml` declares an action
whose `[actions.<id>.executor]` table sets `kind` to a string
the runtime doesn't dispatch. Either a typo (`brwoser` instead
of `browser`), a hand-edit against a renamed variant, or a
manifest authored against a future binary that has executors
this one doesn't.

**Fix**

1. Open the manifest at the path in the error
   (`actions.draft_reply.executor.kind` → the
   `[actions.draft_reply.executor]` table inside `vertical.toml`).
2. Replace `kind = "nonsense"` with one of the valid kinds
   listed in the error's `valid:` clause. The canonical list
   lives in
   [`crates/argo-vertical/src/manifest_validate.rs`](../../../crates/argo-vertical/src/manifest_validate.rs)
   (`KNOWN_EXECUTOR_KINDS`).
3. If the action genuinely needs an executor this binary
   doesn't have, upgrade the binary
   (`cargo install --path crates/argo-cli` from a fresh
   checkout); the executor variant may be on a newer release.
4. Re-validate:
   ```bash
   argo vertical validate path/to/vertical
   ```
   Expected: no errors, exit 0.

**Related** —
[Observability §9.6](observability.md#96-common-failure-modes)
(the `executor 'computer_use' is not yet implemented` runtime
form, which fires after manifest validation when the executor
is declared but its env vars aren't set).
