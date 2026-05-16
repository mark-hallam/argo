# Discord moderation rules

Server rules + the enforcement ladder operators run when a rule is
broken. Pin the "Rules" section as an embed in `#general`. The
"Enforcement" section is operator-internal — do not pin it.

## Rules (pinned in #general)

```
SKYCOT community rules

1. Be civil. No personal attacks, harassment, slurs, or doxxing.
   The vibe is "calm capable senior colleague" — keep it there.

2. Stay on topic per channel. #help is for questions, #showcase
   for what you've built, #dev-talk for building with the platform,
   #feature-requests for product ideas, #general for everything
   else.

3. No spam. No DM solicitation, no unsolicited promotion of other
   products or services, no recruiter messaging without operator
   approval, no link dumps without context.

4. No piracy, illegal content, or NSFW. SKYCOT runs commercial
   workloads for real customers; this server is the same standard.

5. Don't share other members' private information. Anything posted
   in a DM stays in that DM unless the author re-posts publicly.

6. Don't post raw secrets. Strip tokens, API keys, internal URLs,
   and tenant ids before pasting logs or configs. The bot may
   delete obvious leaks on sight.

7. Bug reports go in #feature-requests with reproduction steps.
   Security issues go to security@skycot.com, not the server.

8. The operator team has final say. If you disagree with a call,
   DM @Operator — don't litigate in-channel.
```

## Enforcement ladder (operator-internal)

Apply the lowest step that resolves the situation. Escalate only on
repeat or severity.

1. **In-channel correction.** Quote the rule, redirect to the right
   channel. ~80% of cases stop here.
2. **DM warning.** First explicit warning, logged in the operator
   notes channel with member id, rule, and a link to the offending
   message.
3. **Timeout, 24h.** Discord's built-in timeout. Use for repeated
   off-topic posting, mild spam, or single low-severity rule break
   after a DM warning.
4. **Kick.** Use for first-strike high-severity (harassment,
   doxxing, single egregious slur). Member can rejoin and start
   fresh if they understand the line.
5. **Ban.** Use for repeat post-kick offenders, deliberate secret
   leaks, illegal content, or coordinated brigading. Bans are
   logged in the operator notes channel with a one-line reason.

## Special cases

- **AI-generated spam / drive-by promotion.** Delete + ban on sight.
  No warning, no timeout — these accounts won't return as legitimate
  members.
- **Security disclosure attempted in public channel.** Delete the
  message, DM the reporter with the security@skycot.com address and
  a thank-you, log internally. Don't timeout — the reporter is
  acting in good faith.
- **Customer reporting an outage in #help during an active
  incident.** Acknowledge, redirect to the status page link, and let
  it ride. Don't enforce the off-topic rule against a paying
  customer during a real incident.
