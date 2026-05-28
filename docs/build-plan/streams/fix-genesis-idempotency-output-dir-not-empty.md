# fix-genesis-idempotency-output-dir-not-empty

**Severity**: 🟡 P1 — MVP demo polish (paired with `fix-spa-genesis-fetch-fails-while-curl-succeeds`)

**Authored against**: `origin/main` SHA `718fb9f6d…` on 2026-05-28T03:50Z.

**Discovered during**: 2026-05-28 e2e walkthrough — Genesis SSE error event after the first successful run for a given tenant+persona combo.

## Symptom

Second `POST /api/v1/genesis` with the same `tenant_id` + `role_description` (which resolves to the same persona slug) emits an SSE error event mid-stream:

```
event: error
data: {
  "message": "write vertical: output dir 'verticals/.genesis/<tenant>/<persona>' is not empty; refusing to overwrite an existing vertical",
  "phase": "generating_skill_bundle",
  "recoverable": false
}
```

The check fires during the `generating_skill_bundle` phase (~65%) — after archetype + charter are compiled. SSE response was 200 OK initially (progress events stream cleanly through phases 1-3), then the error event closes the stream.

## Why this matters for the demo

The demo MP4 will be recorded against a fresh tenant slug, so the first attempt should succeed. BUT:

1. **Pre-record dry-runs**: operator likely re-runs Genesis multiple times during dry-runs to test the flow. The second attempt onwards fails until the dir is manually deleted.
2. **Retry semantics**: if the SPA "Try again" button is clicked after an unrelated network error, Genesis errors with this idempotency message instead of completing — degrades the recovery story.
3. **Investor demo Q&A**: if the operator runs Genesis twice (e.g. "what if I describe a different role?") they may hit this.

## Acceptance criteria

Either path A or path B is sufficient:

### Path A — overwrite mode (simpler)

- [ ] Genesis accepts an `overwrite: true` field in the request body
- [ ] When `overwrite=true`, the `verticals/.genesis/<tenant>/<persona>/` directory is wiped before writing the new vertical
- [ ] Default behavior (no `overwrite`) preserves the current "refuse to overwrite" safety
- [ ] SPA sets `overwrite: true` for retry attempts but not for fresh attempts

### Path B — unique persona suffix per attempt (more idempotent)

- [ ] Genesis derives a unique `<persona>-<timestamp>` or `<persona>-<short-hash-of-description>` slug for each attempt
- [ ] Same description → same suffix → idempotent re-runs land on the same dir
- [ ] Different description → different suffix → new dir, no conflict

## May-touch

- `crates/argo-genesis/src/write_vertical.rs` (or wherever the "output dir not empty" check lives — grep for the error message text)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs` — possibly accept new `overwrite` field
- `web/lib/api/genesis.ts` — possibly send `overwrite: true` on retry

## Must-not-touch

- The Genesis pipeline output format — preserve the manifest / prompt / skill-bundle shape

## References

- Discovered during walkthrough at `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 3
- Backend Genesis is operator-promoted MVP-must-have (memory: `genesis-is-mvp-must-have-2026-05-28`)
- Companion brief: `fix-spa-genesis-fetch-fails-while-curl-succeeds` (blocks the SPA-side Genesis flow entirely; this brief covers the second-attempt re-run case)
