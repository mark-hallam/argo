# Operator quiet hours — workspace-level

Brief: `mission-quiet-hours-channels` (MISSION_BACKLOG item #121).

Enterprise customers (and operators with sleep schedules) want noisy
interrupt channels — SMS, voice, WhatsApp, iMessage, push — gated by a
nightly window so dispatch falls through to a non-noisy channel
(typically WebInbox) until morning. SKYCOT honours this via a single
`[operator.quiet_hours]` block in `argo.toml`.

This is the **workspace-level** surface. There is also a per-operator
`[operator.<id>.preferences.quiet_hours]` block (see
`argo_channels::operator_preferences`) which masks specific channels
for one operator and a per-channel `quiet_hours = …` knob under each
`[channels.<kind>]` table that defers `send` at the channel layer.
The three layers compose; this doc covers the workspace surface only.

## Config shape

```toml
[operator.quiet_hours]
enabled = true
start = "22:00"   # HH:MM in the operator's local time
end   = "07:00"   # wraps midnight when end < start
timezone = "Australia/Melbourne"  # IANA timezone name
noisy_channels = ["sms", "voice", "whatsapp", "imessage", "push"]
```

If the block is absent or `enabled = false`, the dispatcher behaviour
is unchanged from baseline — the fallback chain reaches per-channel
send in the order produced by
`mission-channels-operator-preferred-fallback::promote_preferred`.

## Field reference

| Field            | Type         | Default                                                  | Notes                                                                                            |
| ---------------- | ------------ | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `enabled`        | `bool`       | `false`                                                  | Master switch. `false` ⇒ this filter is a no-op.                                                 |
| `start`          | string       | `"22:00"`                                                | Window start, `HH:MM`, in the operator's local time.                                             |
| `end`            | string       | `"07:00"`                                                | Window end, `HH:MM`, in local time. When `end < start` the window wraps midnight.                |
| `timezone`       | string       | `"UTC"`                                                  | IANA timezone name (e.g. `"Australia/Melbourne"`, `"America/Los_Angeles"`).                      |
| `noisy_channels` | array&lt;string&gt; | `["sms", "voice", "whatsapp", "imessage", "push"]` | Channel kinds skipped while the window matches. Anything not in this set rings through.          |

## DST and timezone semantics

The window is interpreted in the **local time** of the configured
`timezone`. SKYCOT uses `chrono_tz` under the hood — daylight-saving
transitions are honoured automatically. A `22:00 → 07:00` window in
`Australia/Melbourne` stays at `22:00 → 07:00` local across the
AEST/AEDT switch (UTC offset changes; local wall clock does not).

Invalid timezone strings (`timezone = "Mars/Olympus"`) collapse the
resolver to a disabled state — the filter becomes a no-op instead of
failing the dispatch. This matches the operator surface contract that
quiet hours is an opt-in **override**, never a hard requirement.

## Default `noisy_channels` rationale

The shipped default covers the five interrupt channels that wake an
operator at home:

- `sms` — text message → phone-level alert
- `voice` — phone call → loudest possible interrupt
- `whatsapp` — push to phone
- `imessage` — push to phone
- `push` — Web Push / mobile push notifications

`slack` is **not** in the default list. Slack DMs follow the
operator's Slack-app DND status at the OS / Slack layer; double-muting
them here usually causes more confusion than it prevents. Operators
who want Slack quiet too can override:

```toml
noisy_channels = ["sms", "voice", "whatsapp", "imessage", "push", "slack"]
```

## Empty-chain fallthrough

When every channel in the fallback chain is in `noisy_channels` AND
the window matches, the filter would otherwise empty the chain. To
preserve the "approval is never silently lost" floor guarantee
(`webinbox` is the always-on JSONL writer), the resolver appends
`"webinbox"` as a single fallthrough step.

Example: `chain.order = ["sms", "voice", "push"]` at 02:00 local with
the default config becomes `["webinbox"]`. The operator sees the
approval in the inbox at sunrise rather than being woken by SMS.

If you do **not** want WebInbox to be the fallthrough — e.g. you want
the dispatch to fail loudly so escalation can pick it up — keep at
least one non-noisy channel (typically email) in the chain so the
filter never empties.

## Integration points

The filter runs in `argo-cli/src/handlers/worker_actions_notify.rs::try_dispatch_on_ask`,
AFTER `worker_actions_notify_fallback::promote_preferred` has rotated
the operator-preferred channel to position 0. Order matters:

```
chain → promote_preferred(chain, preferred) → quiet_hours.filter_chain(chain, now) → dispatch_with_fallback(…)
```

This means the operator-preferred channel (Slack-first, etc.) is
evaluated against the quiet-hours filter just like any other entry.
If your preferred channel is SMS and you're inside the window, SMS is
stripped — the chain falls through to the next promoted step.

## Future work (deferred to v1.x)

Per the brief, this lands the **skip** behaviour only. The richer
alternative — queue the notification for delivery at the start of the
next non-quiet window — is deferred. When that ships, the config
shape will likely grow a `policy = "skip" | "queue"` discriminator,
defaulting to `skip` for back-compat.
