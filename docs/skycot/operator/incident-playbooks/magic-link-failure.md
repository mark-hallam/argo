# Magic-link email delivery failure

**Severity**: P1 (signup + login blocked = revenue impact).

## Detection

- Postmark / SendGrid webhook reports bounce rate > 5% over a
  15-minute window, OR
- Multiple support tickets mention "didn't receive my login link", OR
- Signup funnel drops sharply with no corresponding gateway error.

## Triage (≤5 min)

1. Check the email provider dashboard for delivery status.
2. Confirm sender domain SPF / DKIM / DMARC still resolve (provider
   dashboards surface this when broken).
3. Inspect `[email.outbound]` in `~/.argo/argo.toml` (or deployed
   equivalent): provider, API-key slot, from-address, reply-to.
4. Send a test magic-link to an operator-controlled inbox:

   ```bash
   argo auth magic-link --email <operator-test-inbox>
   ```

## Mitigation

- **Provider outage**: failover to backup email provider by flipping
  `[email.outbound].provider` and `argo gateway reload`. Backup
  credentials live in the same keyring slot scheme.
- **Sender-reputation drop**: pause signup at the SPA edge (serve the
  maintenance banner); coordinate with provider support to clear
  reputation.
- **Credential expiry**: rotate via
  `argo cred set --name <slot> --value-stdin`, then restart gateway.

## Rollback

```bash
git -C ~/.skycot/config log --oneline -5
git -C ~/.skycot/config revert <bad-sha>
argo gateway reload
```

If the issue began after a config change, the prior config is the
fastest restore.

## Notes

- Magic-link tokens expire after the configured TTL; tell affected
  customers to request a fresh one rather than re-use stale ones.
- Use placeholders (`<tenant-acme>`, `<operator-test-inbox>`) in any
  incident-log examples; never embed real customer emails.
