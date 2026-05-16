# claude-discord-community-setup — Discord community + bot + moderation (Wave 4 P1)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Scope

P1. Discord community at `discord.gg/skycot` (or similar). This brief delivers:
1. Discord server template (operator instantiates).
2. Bot for status updates + welcomes.
3. Moderation rules + welcome message.
4. Channel structure: #welcome, #general, #announcements, #help, #showcase, #feature-requests, #dev-talk, #status.
5. Link integration: marketing site footer + SPA in-app footer.

LOC: ~150 (mostly docs + bot config).

## Files you may touch

- `docs/skycot/community/discord-setup.md` (NEW) — operator runbook for setting up the server. ≤120 LOC.
- `docs/skycot/community/welcome-message.md` (NEW). ≤50 LOC.
- `docs/skycot/community/moderation-rules.md` (NEW). ≤80 LOC.
- `marketing/components/SocialLinks.tsx` (depends on `claude-marketing-site-skycot-com` MERGED; that brief creates the file) — narrowly: ensure Discord link configured. ≤15 modified lines.
- `web/components/ui/Footer.tsx` (depends on `claude-design-system-component-extension` MERGED; that brief creates the file) — narrowly: same. ≤10 modified lines.

## Acceptance criteria

- [ ] Operator runbook is complete (5+ steps to live).
- [ ] Welcome message + moderation rules documented.
- [ ] Discord link present in marketing footer + SPA footer.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-discord-community-setup`
- PR title: `community: Discord setup + bot + moderation (claude-discord-community-setup)`
