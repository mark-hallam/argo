# Wiring AWS SES so magic-link sign-in emails actually send (LAUNCH-1)

**Status:** the gateway code now supports real TLS SMTP delivery (commit shipped
2026-06-09). The default is still `noop` (links only get logged), so this is the
**one operator step** that flips magic-link email from "copy the link out of the
log" to "the email lands in the user's inbox."

You (Mark) do the AWS + credential steps — I never enter credentials. Once the
config below is in place and the gateway is restarted, sign-in emails send.

## Why this was the blocker

The previous SMTP sender was plain-TCP with **no TLS**. AWS SES (and Gmail, and
every real relay) rejects plaintext sessions, so the only working delivery mode
was `noop` (log the link). The fix routes SMTP through `lettre` with mandatory
TLS — STARTTLS on port 587, implicit TLS on 465 — so SES now works.

## One-time AWS SES setup (operator)

1. **Pick a region.** For Melbourne, `ap-southeast-2` (Sydney) is the closest SES
   region. Any SES region works; just match it in `smtp_host` below.
2. **Verify your sender.** In the SES console → *Verified identities*, verify the
   domain you'll send from (preferred — lets you send from any address at that
   domain) or at minimum the single `from_address`.
3. **Leave the sandbox.** New SES accounts are sandboxed (can only send to
   *verified* recipients). Request *production access* so you can email any
   signing-up user. Until then, only verified test recipients receive mail.
4. **Create SMTP credentials.** SES console → *SMTP settings* → *Create SMTP
   credentials*. This yields an **SMTP username** and **SMTP password** (these are
   IAM-derived, distinct from your AWS access keys). Save them — the password is
   shown only once.

## Gateway config

Set the `[email.outbound]` block in the gateway's `argo.toml` (the one the live
gateway loads), substituting your region, verified sender, and the SMTP
credentials from step 4:

```toml
[email.outbound]
provider     = "smtp"
from_address = "noreply@your-verified-domain.com"
smtp_host    = "email-smtp.ap-southeast-2.amazonaws.com"   # match your SES region
smtp_port    = 587                                          # 587 = STARTTLS (recommended); 465 = implicit TLS
smtp_username = "<SES SMTP username>"
smtp_password = "<SES SMTP password>"
```

Notes:
- `from_address` MUST be a verified SES identity (step 2), or SES rejects the send.
- Port `587` uses STARTTLS; `465` uses implicit TLS. Both are supported.
- Credentials are stored `Redacted` in config, so they never leak into config dumps.

## Restart + verify (live)

1. Restart the gateway so it loads the new config + the TLS SMTP code:
   `bash scripts/restart-live-gateway.sh --force --no-prompt`
2. Trigger a sign-in: request a magic link for a (verified, if still in sandbox)
   address.
3. **Verify in the log:** the gateway should log
   `outbound email dispatched (TLS)` (target `gateway::email_outbound`). A failure
   logs `smtp send: ...` with the SES error — the signup still returns 200 (no
   retry, silent-fail stance), so always check the log on first setup.
4. Confirm the email arrives and the link signs you in.

## What's verified vs not (NO THEATER)

- ✅ **Verified:** the TLS SMTP code compiles, unit tests pass
  (`email_outbound_tests.rs`, incl. malformed-from rejection), and the provider
  switch + message build are correct.
- ⏳ **NOT yet verified (needs operator):** an actual end-to-end SES send — that
  requires your SES credentials + a verified sender + production access, which
  only you can provision. Once the config above is live, the log line in step 3
  is the proof.

## Alternative: Postmark / SendGrid (already supported, no code change)

If SES setup is slow, the gateway already supports Postmark and SendGrid via
their HTTPS APIs — set `provider = "postmark"` (+ `postmark_token`) or
`provider = "sendgrid"` (+ `sendgrid_api_key`) instead. Same restart + verify.
