# fix-web-extract-ssrf-guard

**Authored against**: origin/main `b11b3f5c2ecadb5471fe9b17899c15405d7a9acd` on `2026-06-11`
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-web-extract-ssrf-guard

## Goal (P1 security — blocks cu1d/#2819)

`web_extract` (and any web tool that fetches a model-supplied URL) is an SSRF
hole: `fetch_and_extract` (`crates/argo-tool/src/tools/web/extract.rs:16-24`)
does `client.get(url).send()` on the **raw model-supplied URL** with NO scheme
check, NO IP-resolution block, and NO redirect cap. A caller can fetch
`http://169.254.169.254/latest/meta-data/iam/security-credentials/` (cloud IAM
creds), `http://127.0.0.1:7400/` (the gateway's own API), or any RFC-1918
internal host — body returned to the model. Route every model-URL web fetch
through an SSRF guard.

## Why now

cu1d (#2819) grants `NetEgress{**}` to fenced workers whose manifest allows
`web_extract` — making this SSRF reachable by least-privilege (potentially
prompt-injected) worker teammates, the exact adversary the capability fence
exists to contain. #2819 is HELD pending this fix. (Adversarial review of #2819,
2026-06-11.)

## Fix

1. **Lift the existing guard into a shared helper.** `crates/argo-genesis/src/safe_fetch.rs`
   ALREADY implements the defense: scheme allow-list, pre-connect
   `is_blocked_ip` (blocks loopback / private / link-local / 169.254.169.254
   metadata), and per-hop redirect re-validation. Extract `is_blocked_ip` +
   the scheme allow-list + the manual-redirect-revalidation into a shared
   location both `argo-genesis` and `argo-tool` can use (a small shared crate,
   or `argo-tool` exposes a `safe_fetch_text` the web tools call — pick the
   layering that doesn't create a bad dep direction; grep before adding a dep).
2. **Route the web tools through it**: `web/extract.rs` (`fetch_and_extract`),
   and audit `web/search.rs` + `vision.rs` for the same raw-fetch pattern —
   any model-supplied-URL fetch goes through the guard.
3. **Tests**: `web_extract` (and siblings) REFUSE `169.254.169.254`,
   `127.0.0.1`/`localhost`, and an RFC-1918 host (e.g. `10.0.0.5`,
   `192.168.1.1`), and a redirect that lands on an internal host. Mirror
   `safe_fetch_tests.rs`.

## May touch
- `crates/argo-tool/src/tools/web/{extract.rs,search.rs,mod.rs}`, `vision.rs`
- `crates/argo-genesis/src/safe_fetch.rs` (extract the guard) OR a new shared helper
- the web-tool tests
- after merge: unblock cu1d (#2819) BLOCKED→TODO

## Acceptance / verifier
1. `web_extract` against a metadata-IP / loopback / RFC-1918 host → refused
   (error, no fetch), proven by tests.
2. A normal public-host extract still works (no regression).
3. One shared SSRF guard — `rg "client.get".*url` shows no raw model-URL fetch
   bypassing it in the web tools.
4. Gate green.

## Size / owner
~150-250 LOC (guard-extract + wiring + tests). Owner: claude. 🔴 P1 — live SSRF
to cloud-metadata + the gateway, weaponized for workers by the held #2819.
