# argo-mcp-connector-signal

Argo MCP connector for Signal messaging routed through the
[`signal-cli`](https://github.com/AsamK/signal-cli) local HTTP/JSON-RPC
daemon. Signal is the operator-requested high-privacy approval channel —
sister to iMessage (per-device) but cross-platform and end-to-end-encrypted
by default. Used by HIPAA-adjacent vertical pilots that require Signal as
the approval channel for sensitive workflows.

## Architecture

The connector ONLY speaks to a loopback signal-cli daemon (default
`http://127.0.0.1:8080`) and never reaches Signal's infrastructure
directly. Cryptographic operations, identity-key storage, and the Signal
wire protocol are entirely signal-cli's responsibility.

```text
agent ──► argo-mcp-connector-signal ──► signal-cli (HTTP daemon) ──► Signal
                                       (loopback only)
```

## One-time setup

The operator runs signal-cli once per host. The connector assumes this is
already in place; it does not bundle, install, or supervise signal-cli.

1. Install signal-cli (Homebrew on macOS, distro package on Linux, or the
   [release tarballs](https://github.com/AsamK/signal-cli/releases)):

   ```bash
   brew install signal-cli           # macOS
   # or unpack the tarball into /opt/signal-cli on Linux
   ```

2. Register or link an account. Linking attaches signal-cli to an existing
   Signal install on a phone — preferred for service accounts:

   ```bash
   signal-cli link -n "argo-signal"   # prints a tsdevice:/?... URI
   # scan the URI from Signal on the phone (Settings → Linked devices)
   ```

   Alternatively, register a fresh number with SMS verification:

   ```bash
   signal-cli -a +15551234567 register
   signal-cli -a +15551234567 verify <code-from-sms>
   ```

3. Start the local HTTP daemon. Bind to loopback only:

   ```bash
   signal-cli -a +15551234567 daemon --http 127.0.0.1:8080
   ```

   For long-running deployments wrap this in a systemd unit or launchd
   plist that restarts on failure. Sample systemd unit:

   ```ini
   [Unit]
   Description=signal-cli daemon for argo
   After=network-online.target

   [Service]
   ExecStart=/usr/bin/signal-cli -a +15551234567 daemon --http 127.0.0.1:8080
   Restart=on-failure
   User=signal

   [Install]
   WantedBy=multi-user.target
   ```

## Connector use

```rust
use argo_mcp_connector_signal::SignalConnector;

# async fn doc() -> argo_types::ArgoResult<()> {
let signal = SignalConnector::new(
    "http://127.0.0.1:8080".into(),
    "+15551234567".into(),
);
let receipt = signal.send_message("+15557654321", "approve?").await?;
println!("ts = {}", receipt.timestamp);

for env in signal.receive().await? {
    if let Some(body) = env.body {
        println!("{}: {body}", env.source_number);
    }
}
# Ok(()) }
```

## Out of scope for v1

- Bundled signal-cli install.
- Group create / admin (only list + send-to).
- Attachment upload (text only for v1).
- Registration / linking flow (operator does this once via `signal-cli link`).
- Webhook-style push from signal-cli (poll-only).
- Gateway dispatcher wiring (follow-up brief).

## Integration smoke test

`tests/signal_smoke.rs` runs against a live daemon when three env vars are
set; it silently skips otherwise. To execute it on a host with signal-cli
already daemonized:

```bash
export ARGO_SIGNAL_DAEMON_URL=http://127.0.0.1:8080
export ARGO_SIGNAL_ACCOUNT=+15551234567
export ARGO_SIGNAL_TEST_RECIPIENT=+15557654321
cargo test -p argo-mcp-connector-signal --test signal_smoke -- --nocapture
```

The test sends one message and polls `receive` once; it asserts the send
returned a positive envelope timestamp, not end-to-end delivery
acknowledgement (which would require a second registered account).
