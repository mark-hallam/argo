# Follow-up templates

Auto-approval-safe templates used by the `follow_up_lead` action.
The action body MUST come from one of the named templates below; the
auto-approval safety case rests on the body being templated, not
generated. Custom messages go through `send_listing_packet` instead.

Merge fields: `{{ lead_name }}`, `{{ listing_address }}`,
`{{ agent_name }}`. Substitute exactly — do not paraphrase, expand,
or add new sentences.

## Template: `post_viewing_24h`

Subject: Following up on your viewing — {{ listing_address }}

Hi {{ lead_name }},

Hope you're well. Just following up on your viewing of
{{ listing_address }} yesterday — I'd love to hear what stood out for
you and whether anything is worth a closer look.

If you'd like to step through the listing again, see another in the
same band, or talk through next steps with {{ agent_name }}, just
reply and I'll set it up.

Thanks,
Jamie, on behalf of {{ agent_name }}

## Template: `post_viewing_72h`

Subject: Quick check-in on {{ listing_address }}

Hi {{ lead_name }},

Wanted to check in on {{ listing_address }}. No pressure either way —
just curious if it's still on your shortlist or if you'd like me to
queue up a couple more options for you.

Happy to share comparable listings or schedule a second look with
{{ agent_name }} if that helps.

Thanks,
Jamie, on behalf of {{ agent_name }}

## Template: `cold_lead_revive`

Subject: Still looking? A quick re-engagement from {{ agent_name }}'s desk

Hi {{ lead_name }},

It's been a little while since we last connected. The market has
shifted a bit, and a few new listings in your range have come up
since.

If you're still actively looking, I can send a refreshed packet that
matches what you told us before. If you've paused, that's fine too —
just reply and I'll archive your file so you don't get further
follow-ups until you're ready.

Thanks,
Jamie, on behalf of {{ agent_name }}
