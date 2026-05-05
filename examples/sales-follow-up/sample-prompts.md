# Sales Follow-up — sample lead prompts

5 follow-up scenarios that produce good worker output against
the `sales-follow-up` vertical.

The pattern that works:

1. **Lead identity** — full name + company + role.
2. **Last touch** — when did the operator last contact them,
   and via what channel?
3. **Specific signal** — what new thing happened that justifies
   this follow-up? URL preferred.
4. **Cadence position** — first follow-up since X / second / third?
5. **What to draft** — short check-in / specific question / share-this-resource pattern.

---

## post-demo nudge

```
Lead: Sarah Patel, VP Engineering at Cohort Labs (cohortlabs.example).
Last touch: gave her a 30-min product demo 9 days ago via Zoom.
Signal: she posted on LinkedIn yesterday about scaling pains in
their data pipeline (https://www.linkedin.com/feed/update/sarah-patel-cohort).
Cadence: first follow-up.
Draft a check-in that references the post + offers to share our
case study from a similar customer.
```

Expected: 1× research_lead (verify role), 1× draft_followup
(autonomous, queues a 4-line draft), 1× send_followup_email
(approval-gated, awaiting approve).

## post-trial check-in

```
Lead: Hideo Tanaka, Eng Director at Kestrel.
Last touch: started a 14-day trial 12 days ago. We saw 4
sessions in week 1, then nothing for 5 days.
Signal: trial expires tomorrow; usage gap is unusual for
this profile.
Cadence: second follow-up (we sent a "checking in" email at
day 7).
Draft a short note offering a 15-min call to unblock whatever
stopped them.
```

Expected: 1× research_lead (recent activity check), 1×
draft_followup (autonomous), 1× send_followup_email (approval-gated).

## role change signal

```
Lead: Megan Liu, formerly Product Manager at Tilt — left 2 weeks
ago, just announced new role as Director of Product at Boltworks
(boltworks.com).
Last touch: 6 weeks ago at Tilt; conversation was warm but the
project paused when budget cycle ended.
Signal: new company + new title. Boltworks is a likely fit
profile-wise.
Cadence: re-introduction (technically first touch at Boltworks).
Draft a short note congratulating her on the role + asking if
the new team has the same problem space.
```

Expected: 1× research_lead (verify Boltworks profile), 1×
find_contact (verify new email), 1× draft_followup (autonomous),
1× send_followup_email + 1× log_to_crm (both approval-gated).

## podcast signal

```
Lead: Carlos Mendez, CTO at FreightFlow.
Last touch: 3 months ago — we exchanged 2 emails, no demo.
Signal: he was a guest on the Modern Logistics podcast 4 days
ago (https://podcasts.example.com/modern-logistics/freightflow).
He talked about wanting "more proactive monitoring" — exactly
what we sell.
Cadence: third follow-up (at 4-week intervals so far).
Draft a note referencing his podcast comment + offering a
demo.
```

Expected: 1× research_lead (verify podcast claim), 1×
draft_followup (autonomous), 1× send_followup_email (approval-gated,
HIGH RISK because it's a third touch — operator should confirm
the cadence isn't getting noisy).

## dropped lead — handle with care

```
Lead: Andy Powell, Head of Ops at QuickShip.
Last touch: 5 follow-ups across 6 months, no replies after touch
2. Final email was 3 weeks ago.
Signal: nothing new. We're trying to decide whether to keep
trying or drop.
Cadence: would be fifth follow-up.
Decide: should we drop this lead?
```

Expected: `[STOP: NEED_INFO drop this lead?]`. The worker's
hard rule is "don't auto-escalate cadence past 4 touches".
Surfaces the decision to the operator + recommends drop +
references the no-reply pattern. NO actions emitted.
