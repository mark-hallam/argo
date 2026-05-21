# Operator action log

**Operator-maintained.** A lightweight diary of what you actioned, posted, skipped, or declined from the agents' outputs. Agents read the tail of this file (last 50 lines) for feedback signal so they can self-correct.

Format: free-text per day. Discipline is optional — even sporadic entries help. Empty is fine for the first week.

---

## Recommended day format

```
## YYYY-MM-DD
- posted: drafts/YYYY-MM-DD/linkedin-personal-process.md (LinkedIn, 14:30 Melbourne)
- skipped: drafts/YYYY-MM-DD/x-personal-process.md (too dev-jargon-y)
- actioned: auditor LATEST.md P0 #2 (cookie middleware fix landed in commit abc1234)
- declined: auditor LATEST.md P1 #4 (deferred — not ready, will revisit next week)
- noted: <anything else worth telling the agents>
```

Verbs the agents recognize:
- `posted:` — marketing draft went out to the named channel
- `skipped:` — marketing draft was reviewed but not posted
- `actioned:` — auditor recommendation was implemented (cite commit / PR if you remember)
- `declined:` — auditor recommendation reviewed + rejected (give 1-line reason so the auditor doesn't repeat)
- `noted:` — anything else (a customer reply, a market signal, an internal decision)

The agents are not judging your output rate — even a single weekly entry gives them enough to self-correct.

---

## Entries

(begin)
