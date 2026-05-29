# DEC-001 — Enterprise focus, not consumer

- **Date**: 2026-05-14
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
SKYCOT serves enterprise customers, not consumers. MCP connector priority,
authoring decisions, and scope calls reflect this. The 4 consumer/redundant
connector stubs (signal, telegram, mailgun, resend) are deleted; the 3
enterprise stubs (apollo, zoominfo, zoom) stay as DEFERRED-v1.x placeholders.

## Rationale
Enterprise is the target market + buyer. When in doubt about a connector or
feature, ask "is this enterprise?"

## How to apply
Connector + feature scope decisions favour enterprise use-cases. Reject
consumer-flavoured scope creep unless it serves an enterprise journey.
