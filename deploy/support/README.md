# Support ticket queue — operator runbook

The SPA help widget (`web/components/ui/HelpWidget.tsx`) POSTs to
`/api/support/ticket`, which forwards to a webhook URL configured per
deployment. Pick one provider for v1; both are listed below.

## Required env vars

| Variable                 | Required | Notes                                              |
|--------------------------|----------|----------------------------------------------------|
| `SUPPORT_WEBHOOK_URL`    | yes      | Provider webhook endpoint (Plain or Linear).        |
| `SUPPORT_WEBHOOK_TOKEN`  | optional | If set, sent as `Authorization: Bearer <token>`.    |

The Next.js route returns `503 support_unconfigured` until
`SUPPORT_WEBHOOK_URL` is set, so the widget surface fails closed.

## Option 1 — Plain.com (recommended)

1. Create a workspace at <https://app.plain.com> and add `support@skycot.com`
   as the inbound email address.
2. In **Settings → Developers → Webhooks**, create an "Incoming" webhook.
   Plain returns a URL like `https://api.plain.com/inbound/...`.
3. Set `SUPPORT_WEBHOOK_URL` to that URL. If Plain issues a signing
   secret, set `SUPPORT_WEBHOOK_TOKEN` to it.
4. Configure email forwarding for `support@skycot.com` → the Plain
   workspace inbound address.

The Next.js route forwards `{ email, message, context }` where
`context` includes `gateway_token`, `user_agent`, `client_ip`, and
`submitted_at`. Plain will create a customer + thread per payload.

## Option 2 — Linear

1. In Linear, open the destination team → **Settings → API → Webhooks**
   → "Create webhook".
2. Subscribe to **Issue** events and copy the URL.
3. Set `SUPPORT_WEBHOOK_URL` to a thin adapter (Cloudflare Worker or
   Vercel function) that converts the SPA payload into a Linear
   `issues.create` GraphQL mutation. Linear's webhook is outbound-only;
   it does not accept inbound issue creation directly.
4. The adapter should authenticate with a personal API key
   (`Authorization: Bearer lin_api_...`), set as `SUPPORT_WEBHOOK_TOKEN`.

## Email forwarding

Configure `support@skycot.com` to forward to the chosen provider's
inbound address. Replies travel via email, so users see a single
threaded conversation in their inbox.

## SLAs

Per-tier response SLAs live at `docs/skycot/support/sla.md` and are
linked from the in-app widget.
