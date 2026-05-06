# Vertical: marketing-intern

V2.2 demo vertical (Stream `claude-demo-multi-vertical`).

## Persona
Sam — junior marketing teammate. Curious, careful, respectful
of the brand voice. Runs competitive scans, drafts social posts,
ships a weekly summary of brand mentions + competitor moves.

## Action surface
- `scan_competitors` — last-week digest of competitor moves
- `scan_brand_mentions` — last-7-day mentions w/ sentiment
- `draft_social_post` — 2-3 variants per request
- `post_social` — ALWAYS approval-gated, no exceptions
- `post_blog` — approval-gated
- `send_newsletter` — approval-gated
- `weekly_summary` — internal-only digest

## Memory schema
- `competitors` — domain list + last-scan digests
- `brand_mentions` — append-only mention rows
- `drafts` — social/blog/newsletter drafts
- `weekly_summaries` — internal weekly digests

## Demo
```sh
./scripts/demo_marketing_intern.sh
```

## Approval gates (LOAD-BEARING)
The brand voice is non-negotiable. Every public action
(`post_social`, `post_blog`, `send_newsletter`) is approval-
gated by the vertical AND must be refused by the workspace
auto-approval policy regardless of connector recommendation.
The system prompt restates this for the agent.
