---
name: research_lead_signals
description: Given a lead context (company + role + last-touch), summarise recent public signals (hiring posts, funding announcements, product launches, role changes) and propose one specific relevance-hook for the next touch. Recommends skip when no fresh signal in 30 days. Drafting only — no external effect.
inputs:
  - lead_context
  - recent_window_days
  - allowed_sources
outputs:
  - recent_signals_summary
  - suggested_relevance_hook
  - skip_recommendation
  - flags
---

# Skill: Research Lead Signals

Surface one specific signal that justifies the next follow-up.

1. **Read the lead context.** `lead_index` row carries
   company, role, stage, last_touch_at. If any of these is
   missing, flag `missing_lead_fields` and stop — never invent
   a role or a tenure.
2. **Scan the recent window.** Default is 30 days; `recent_window_days`
   overrides. Allowed signal classes: hiring posts, funding /
   acquisition announcements, product / feature launches,
   public role changes, podcast / press appearances. Skip
   social posts that don't name a concrete event.
3. **One signal, one source.** Pick the strongest fresh signal,
   not all of them. Carry the source URL — drafting downstream
   depends on a citable link. If two signals are equally strong,
   pick the one closest to the lead's role or stated priorities.
4. **No fresh signal → recommend skip.** Set
   `skip_recommendation = true` with reason
   `"no_fresh_signal_in_<N>d"`. The operator can override but
   the vertical's default is do-not-nurture-spam.
5. **Relevance hook.** One sentence connecting the signal to
   the lead's job-to-be-done — not to our product. "You're
   hiring 3 platform engineers" lands; "You'll love what we
   built" doesn't.

Hard rules:

- **No protected-class or PII signals.** Never surface age,
  gender, family status, religion, health, immigration status.
  If a public source volunteers any of these, ignore them.
- **No live scraping of social profiles.** This skill consumes
  pre-fetched signals from `allowed_sources`; it does not call
  out to LinkedIn, X, or any walled platform. If the operator
  has not supplied a source for a class, return without
  fabricating one.
- **Stale signals don't count.** A funding round announced 11
  months ago is not a fresh signal. The skip-threshold is
  load-bearing — eval fixture #2 exercises this exactly.
- **`next_action` is informational.** Research is autonomous-safe;
  it never sends or schedules. Downstream skills consume the
  signal; only `send_email` / `update_crm` / `schedule_call`
  cross the human-approval gate.
- **If `lead_context.do_not_contact` is true → stop.** Surface
  `flags: ["do_not_contact"]` and escalate. The vertical never
  researches a lead the operator (or the lead themselves) has
  opted out.
