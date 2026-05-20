# web-workflow-trigger-config-ux — tailored config forms for webhook / schedule / email / form triggers

**Owner**: claude. **Sizing**: ~300-500 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-trigger-config-ux`.
**Priority**: **P0** — every n8n / Zapier-class workflow needs a trigger. SKYCOT's builder lets you add trigger nodes today but the config drawer is generic. Without a tailored UX, no real customer can actually wire a workflow to anything.

## Scope

The builder's left-side node palette (or the existing Add-step affordance) already exposes trigger step kinds (`manual`, `schedule`, `webhook`, `email`, `form`, `slack`). When the user selects one, the config drawer should render a **tailored form** per kind. Today the drawer just shows the generic step config. This brief replaces it with per-kind UX.

### 1. Webhook trigger

- Generated URL: `https://api.skycot.com/v1/webhooks/<workflow-id>/<secret-suffix>` displayed read-only in a copy-to-clipboard field.
- Secret rotation: "Regenerate" button (with confirm dialog — old URL stops working immediately).
- Test ping section:
  - "Send test request" button — fires a POST to the URL with a sample JSON body, shows status + response panel.
  - Inline curl snippet the operator can copy.
- Optional auth: header check (key + value) or none.
- Allowed methods: checkbox set (POST default; GET/PUT/etc. optional).

### 2. Schedule trigger

- Cron-picker UI:
  - Top: human-readable mode tabs (Every minute / Every hour / Every day / Every week / Every month / Custom cron)
  - Each tab shows the relevant secondary inputs (hour picker, day-of-week chips, day-of-month, etc.)
  - "Custom cron" tab shows a raw text input + cron-syntax validator + a "Next 5 runs" preview list.
- Timezone selector (defaults to workspace TZ — read from session).
- "Cron explained" inline: "Runs at 9:00am every weekday in Australia/Melbourne".

### 3. Email trigger

- IMAP credentials form:
  - Server (host + port)
  - Username
  - Password (input type="password", masked)
  - SSL/TLS toggle
- Filter rules:
  - From address (optional, multi-line — one address or domain per line)
  - Subject contains (optional)
  - Has attachment (boolean)
- "Test connection" button — gateway probes the credentials and reports success/failure. If endpoint doesn't exist yet, show "Test endpoint coming v1.x" tooltip and let the operator save anyway.
- Note: credentials should reference the gateway secret store (use existing secret-ref pattern if available; otherwise plaintext for v1 with a TODO comment).

### 4. Form trigger

- Field builder:
  - Add field button → asks for label + type (text / textarea / email / number / date / select / checkbox / file)
  - Each field row: label, type, required toggle, placeholder, delete button
  - Drag-reorder field rows (use existing drag library if available; else up/down arrows)
- Form URL preview: `https://skycot.com/forms/<workflow-id>` (read-only)
- Public/private toggle (public = anyone with the URL; private = requires SKYCOT login)
- Submit button label (text input, default "Submit")
- Success message (textarea, default "Thanks! We'll be in touch.")

### Shared infrastructure

- New component `web/components/workflows/triggers/<kind>ConfigForm.tsx` per kind.
- A `TriggerConfigDrawer` wrapper that switches on `step.kind` and renders the matching form.
- Form state pipes back through the existing builder step-config save path (TOML encoder).
- Each form has its own zod schema for validation + inline error display.

## Acceptance criteria

- [ ] Selecting a webhook trigger node opens a drawer with generated URL + copy button + regenerate + test ping button.
- [ ] Webhook test ping POSTs to the generated URL and shows status + response.
- [ ] Schedule trigger drawer shows tabbed cron picker (Every minute / hour / day / week / month / Custom) + timezone selector + next-5-runs preview.
- [ ] Email trigger drawer shows IMAP creds form + filter rules + test-connection button.
- [ ] Form trigger drawer shows field builder + URL preview + public/private toggle + submit label + success message.
- [ ] All four forms save config back to the workflow via the existing step-config save path.
- [ ] Zod validation catches invalid inputs with inline errors.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Backend webhook receiver / cron scheduler / IMAP poller / public form host (gateway concerns — separate briefs).
- OAuth-based email providers (Gmail / Microsoft 365) — v1.x.
- Form file-upload to S3-style storage — v1.x.
- Slack trigger config — has its own existing config flow, not in this brief.
