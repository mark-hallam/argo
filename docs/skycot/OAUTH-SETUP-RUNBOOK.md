# OAuth connectors — operator setup runbook

**What this unlocks:** once you complete these steps, a non-developer customer can click **Connect** on Gmail / Google Calendar / HubSpot / Notion / Salesforce in `/settings/connectors` and grant access in their browser — no API-key paste. (The gateway `/connect` route ships in PR — see "Code side" below; it's *inert* until the env vars here are set.)

**One-time, per provider. ~10 minutes each.** You do this in the provider's developer console; I never see the secrets.

---

## The two things every provider needs

1. A **redirect URI** registered on the OAuth app, set to the gateway callback:
   ```
   https://api.skycot.com/api/v1/connectors/<id>/callback
   ```
   …where `<id>` is `gmail`, `gcal`, `hubspot`, `notion`, or `salesforce`.
2. A **Client ID + Client Secret**, which you put in the gateway env (the env-var names are fixed — see each section).

Also set this once on the gateway so the route builds the right redirect URI:
```
ARGO_OAUTH_REDIRECT_BASE=https://api.skycot.com
```
(If unset, the route derives it from the request Host — fine for prod, but set it explicitly to avoid surprises.)

---

## 1 · Google — Gmail + Google Calendar

> ⚠️ **The one thing tonight can't beat:** Gmail's `gmail.modify` (and Calendar's) scopes are **"sensitive/restricted."** Google requires a **security/verification review of your app** before *external* users may grant them — that's days-to-weeks on Google's side. **Until then, add yourself + any pilot customer as "Test users"** (OAuth consent screen → Testing mode) and it works immediately for those accounts.

Steps (Google Cloud Console → APIs & Services):
1. Create/select a project → **OAuth consent screen** → External → add scopes `gmail.modify` and `calendar` → add Test users (you + pilots).
2. **Credentials → Create Credentials → OAuth client ID → Web application.**
3. **Authorized redirect URIs** — add **both**:
   - `https://api.skycot.com/api/v1/connectors/gmail/callback`
   - `https://api.skycot.com/api/v1/connectors/gcal/callback`
4. Copy the Client ID + Secret. You can use the **same Google OAuth client for both** Gmail and Calendar.

Gateway env:
```
ARGO_GMAIL_CLIENT_ID=<google client id>
ARGO_GMAIL_CLIENT_SECRET=<google client secret>
ARGO_GCAL_CLIENT_ID=<same google client id>
ARGO_GCAL_CLIENT_SECRET=<same google client secret>
```

---

## 2 · HubSpot

1. HubSpot developer account → **Create app** → **Auth** tab.
2. **Redirect URL:** `https://api.skycot.com/api/v1/connectors/hubspot/callback`
3. Add the scope your teammates need (start with `crm.objects.contacts.read`).
4. Copy the Client ID + Secret.

Gateway env:
```
HUBSPOT_CLIENT_ID=<hubspot client id>
HUBSPOT_CLIENT_SECRET=<hubspot client secret>
```

---

## 3 · Notion

1. Notion → **My integrations → New integration → Public integration** (OAuth).
2. **Redirect URI:** `https://api.skycot.com/api/v1/connectors/notion/callback`
3. Copy the OAuth Client ID + Secret.

Gateway env:
```
NOTION_CLIENT_ID=<notion client id>
NOTION_CLIENT_SECRET=<notion client secret>
```

---

## 4 · Salesforce

1. Setup → **App Manager → New Connected App** → enable OAuth.
2. **Callback URL:** `https://api.skycot.com/api/v1/connectors/salesforce/callback`
3. OAuth scopes: **api** + **refresh_token** (offline access).
4. Copy the Consumer Key (=Client ID) + Consumer Secret.

Gateway env:
```
SF_CLIENT_ID=<salesforce consumer key>
SF_CLIENT_SECRET=<salesforce consumer secret>
```
(The route also stores the per-org `SF_INSTANCE_URL` returned at connect — no action needed.)

---

## Where to set the env vars

These go on the **gateway** (ECS task-def `skycot-prod-argo`), the same place as the other gateway secrets. The redirect-base + client ids can be plain env; the **client *secrets* belong in Secrets Manager**, injected like the existing `ANTHROPIC_API_KEY` secret. Once set, the connector's **Connect** button works end-to-end (Google still gated on the review for non-test users).

> Token connectors (Slack, Jira, Stripe, etc.) need **none** of this — they're API-key paste and already work.

---

## How to confirm it works

1. Open `/settings/connectors`, click **Connect** on (say) HubSpot.
2. You bounce to HubSpot's consent screen → Allow → you land back on `/settings/connectors?connector=hubspot&oauth=connected`.
3. The connector flips to **Connected** for your tenant; a teammate with HubSpot in its role can now read/write it.
4. If you see `?oauth=not_configured`, the env vars for that provider aren't set yet.

---

## Code side (what ships in the PR)

- New gateway routes `GET /api/v1/connectors/<id>/connect` (→ 302 to the provider) and `/callback` (→ exchange code, store the token **tenant-scoped**, redirect back to the SPA). Stream-owning, mirrors the existing SSO OIDC handler; reuses the CLI's `build_authorize_url` + `exchange_code`. CSRF-protected via a signed short-lived cookie bound to your session tenant.
- The SPA already has the **Connect** affordance + the `/connect/<id>` URL builder (`web/lib/api/connectors.ts`) — no SPA change needed for the OAuth-5.
- **Inert-until-configured by design:** absent client id → the route bounces back with `oauth=not_configured` instead of a broken redirect.
