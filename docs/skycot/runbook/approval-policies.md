# SKYCOT Operator Runbook — Approval Policies

Per-worker action policy overrides for the worker Settings tab. For
the full table of contents, see [RUNBOOK.md](../RUNBOOK.md).

---

## Per-worker Approval Policies

The worker Settings tab carries an "Approval policy" section backed
by two endpoints (`mission-approval-policy-ui`):

```bash
# List action types + current policy + inferred trust grade:
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  http://127.0.0.1:8734/api/v1/workers/<worker_id>/policies

# Change one action type's policy (operator note is required):
curl -sX PATCH -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"policy":"always-ask","operator_note":"smoke-testing v1"}' \
  http://127.0.0.1:8734/api/v1/workers/<worker_id>/policies/<action_type>
```

Allowed policies: `auto-approve | always-ask | always-refuse |
risk-gated`. `risk-gated` is the default for any action type the
worker hasn't explicitly overridden.

Overrides are stored at `.skycot/worker-policies/<worker_id>.toml`.
Each successful PATCH writes an audit row with payload
`{ worker_id, action_type, old_policy, new_policy, operator_note }`.
PATCH on a paused worker returns 409; resume the worker first.

---

Next: [Troubleshooting](troubleshooting.md).
