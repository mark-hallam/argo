# Approval checklist

- stop_reason: `approval_required`

---

## approval_checklist

### Safe to approve

- [ ] **Meeting-request draft** — calls Mark a "VP Partnerships",
      proposes a 20-minute exploratory chat, asks him to share
      a partnership-deck or one-pager beforehand.
      - sources: <https://www.linkedin.com/in/mark-chen-acme>,
        <https://www.acmecorp.io/team>

### Needs human verification

- [ ] **Founder availability** — meeting-request proposes
      Thursday or Friday afternoon. Operator must confirm the
      founder's calendar before the `send_calendar_invite`
      action fires.
- [ ] **Acme's recent partnership** — research turned up a
      similar partnership announcement Acme made last quarter;
      operator should confirm whether SKYCOT actually wants to
      partner here OR if there's a conflict (e.g. with a
      Series A investor).

### Blocked

- _(none — research evidence sufficient: 4 web_search hits + 3 web_extract pages totalling 6100 chars)_
