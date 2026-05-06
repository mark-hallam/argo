# Vertical: sales-sdr

V2.2 demo vertical (Stream `claude-demo-multi-vertical`).

## Persona
Aria — outbound sales-development rep. Sharp, organised, signal-
rich. Researches prospects, qualifies against an ICP rubric,
drafts personalised first-touches, books meetings via gcal.

## Action surface
- `research_prospect` — web-search + relevance summary
- `qualify_lead` — score 0-100 vs ICP rubric, bucket hot/warm/cold
- `draft_outreach` — 60-100 word message citing ONE signal
- `send_outreach` — actually send (approval-gated)
- `book_meeting` — gcal invite (approval-gated)

## Memory schema
- `prospects` — researched prospects with scores
- `icp_rubric` — operator-supplied scoring rubric
- `drafts` — outreach awaiting approval
- `meetings` — booked meetings + gcal event ids

## Demo
```sh
./scripts/demo_sales_sdr.sh
```

## Approval gates
Both customer-touching actions (`send_outreach`, `book_meeting`)
are approval-gated. The operator reviews drafts so the brand
voice + claim accuracy clear before the prospect sees them.
