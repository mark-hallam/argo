# Discord community — operator runbook

Operator playbook for instantiating the SKYCOT Discord server,
configuring channels and roles, deploying the status bot, and
publishing the vanity invite. Designed to be reproducible: a fresh
operator should be able to land a fully-configured server in under
45 minutes from a cold start.

Companion docs:
- `welcome-message.md` — the bot's onboarding DM + #welcome embed.
- `moderation-rules.md` — server rules + enforcement guidance.

## 1. Create the server

1. Discord desktop → "+" → "Create My Own" → "For a club or
   community" → name `SKYCOT`.
2. Server icon: upload `web/public/icon-512.png` (locked brand mark).
3. Server banner: upload `marketing/public/og.png` once the marketing
   site lands; placeholder solid-indigo banner otherwise.
4. Settings → Server Profile → set tagline to the locked positioning
   sentence from `docs/skycot/brand/IDENTITY.md` (verbatim).

## 2. Roles

Create the following roles, in priority order (top = highest):

| Role | Color | Purpose |
| --- | --- | --- |
| `@Operator` | brand-primary indigo | Skycot staff. Manual assignment. |
| `@MVP` | gold | Early customers + design partners. Manual. |
| `@Builder` | teal | Verified developers shipping with SKYCOT. Manual. |
| `@Member` | neutral | Default after #welcome verification. Bot-assigned. |
| `@Unverified` | dim | Default on join, no channel access except #welcome. |

Permissions: only `@Member` and above can post outside #welcome.
`@Unverified` can read #welcome and react to the gate message.

## 3. Channels

Create the eight channels below, in this order, under a single
"COMMUNITY" category. Channel topic strings should match exactly so
the bot's status embeds render against the right surface.

| # | Channel | Type | Read | Post |
| - | --- | --- | --- | --- |
| 1 | `#welcome` | text | all | bot only |
| 2 | `#announcements` | text | all | `@Operator` only |
| 3 | `#status` | text | all | bot only |
| 4 | `#general` | text | `@Member`+ | `@Member`+ |
| 5 | `#help` | text | `@Member`+ | `@Member`+ |
| 6 | `#showcase` | text | `@Member`+ | `@Member`+ |
| 7 | `#feature-requests` | text | `@Member`+ | `@Member`+ |
| 8 | `#dev-talk` | text | `@Builder`+ | `@Builder`+ |

Pin the welcome embed in `#welcome` and the rules embed (from
`moderation-rules.md`) at the top of `#general`.

## 4. Bot — `skycot-bot`

The bot does two jobs: greet new members (DM + #welcome reply) and
post status updates from the Skycot status page into `#status`.

1. <https://discord.com/developers/applications> → "New Application"
   → name `skycot-bot` → Bot tab → Reset Token → store in 1Password
   under `discord-bot-token`.
2. Enable intents: `Server Members Intent`, `Message Content Intent`.
3. OAuth2 → URL Generator → scopes `bot`, `applications.commands`;
   permissions: `Send Messages`, `Embed Links`, `Manage Roles`,
   `Read Message History`. Use the generated URL to add the bot.
4. Deploy the bot worker (separate repo or container; out of scope
   for this brief — see follow-up brief `claude-discord-bot-deploy`).
   For v1, a single Node.js process on the operator's box running
   `discord.js` + a polling task against the Skycot status page is
   sufficient. Env: `DISCORD_BOT_TOKEN`, `SKYCOT_STATUS_URL`,
   `STATUS_CHANNEL_ID`.
5. Bot message contracts:
   - On member join: send the DM from `welcome-message.md`, post the
     welcome embed in `#welcome`, and assign `@Unverified`.
   - On reaction `✅` to the welcome embed: swap `@Unverified` →
     `@Member`.
   - Every 60s: poll `SKYCOT_STATUS_URL`. On state transition, post
     an embed in `#status` (green for resolved, amber for degraded,
     red for outage).

## 5. Vanity invite + integrations

1. Server Settings → Vanity URL → `skycot` (requires Boost level 3;
   fall back to `discord.gg/skycot-community` shortlink otherwise).
2. Add the invite to:
   - `marketing/components/SocialLinks.tsx` (Discord icon, link).
   - `web/components/ui/Footer.tsx` (SPA in-app footer social row).
   - `docs/skycot/MISSION.md` footer.
3. Server Settings → Widget → enable; copy the embed URL into the
   marketing `/about` page once it lands.

## 6. Launch checklist

- [ ] Roles + channels match the tables above.
- [ ] Bot online, posts to `#status`, responds to `✅` reactions.
- [ ] `welcome-message.md` rendered as the pinned embed in #welcome.
- [ ] `moderation-rules.md` rendered as the pinned embed in #general.
- [ ] Vanity invite live, smoke-tested from an incognito browser.
- [ ] Discord icon linked in marketing footer + SPA footer.
- [ ] Operator bookmarked the audit-log channel.
