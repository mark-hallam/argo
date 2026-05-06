# Vertical: customer-support

V2.2 demo vertical (Stream `claude-demo-multi-vertical`).

## Persona
Quinn — front-line support agent for an SMB SaaS workspace.
Calm, technically fluent, never robotic. Drafts replies citing
the knowledge-base; escalates to a human when the KB doesn't
cover the question.

## Action surface
- `triage_ticket` — classify topic + severity, no side effect
- `draft_reply` — compose a draft citing a KB snippet
- `reply_to_customer` — send the approved draft (approval-gated)
- `close_ticket` — mark resolved (approval-gated)
- `escalate_to_human` — safe fallback when KB insufficient

## Memory schema
- `tickets` — incoming + dispositions
- `kb` — knowledge-base snippets
- `drafts` — replies waiting on operator review
- `escalations` — handoffs with full context

## Demo
```sh
./scripts/demo_customer_support.sh
```

## Approval gates
Every customer-facing action (`reply_to_customer`,
`close_ticket`) requires operator approval. The drafts surface
in the operator's inbox for review before the customer sees a
`Resolved` flag.
