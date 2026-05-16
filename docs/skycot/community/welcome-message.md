# Discord welcome message

Source of truth for the bot's onboarding copy. Two surfaces: the DM
sent on join, and the pinned embed in `#welcome`. Tone follows
`docs/skycot/brand/IDENTITY.md` — sentence case, no exclamation
points, calm and direct.

## DM on member join

```
Welcome to SKYCOT.

A few things before you dive in:

- React with ✅ on the pinned message in #welcome to gain access
  to the rest of the server.
- #help is the right place for product questions. #dev-talk for
  building with the platform. #showcase for what you've shipped.
- Bugs and feature ideas live in #feature-requests — they're read.
- Heads-up: SKYCOT is in active development. Things change.
  #announcements has the running ledger.

Glad you're here.
```

## #welcome pinned embed

Title: `Welcome to SKYCOT`

Description:
```
SKYCOT gives you AI workers you can hire, delegate to, and trust
like a teammate.

React with ✅ below to confirm you've read the rules in #general
and unlock the rest of the server.
```

Footer: `Read #general before you post.`

Color: brand-primary indigo (`#3730a3` until the locked hex lands in
`web/app/globals.css --brand-primary`).
