# Marketplace Seller E2B Demo

This demo drives the `marketplace-seller` vertical through a sandboxed
listing flow and stops before publish with `approval_required`.

## Run

```bash
verticals/marketplace-seller/demo/e2b_e2e.sh --out /tmp/marketplace-seller-e2b-demo
```

The default `fixture` mode replays the checked-in E2B recording fixture and
writes:

- `screenshots/*.png` - six evidence frames for the listing flow.
- `screenshots/*.png.txt` - text descriptions paired with each frame.
- `approval_draft.md` - the operator review payload.
- `evidence.json` - machine-readable evidence summary.

For a live sandbox preflight:

```bash
E2B_API_KEY=... ARGO_BIN=target/debug/argo \
  verticals/marketplace-seller/demo/e2b_e2e.sh \
  --mode live-e2b \
  --out /tmp/marketplace-seller-e2b-demo-live
```

`live-e2b` requires `E2B_API_KEY` or `ARGO_E2B_API_KEY`. It runs
`argo work serve --vertical marketplace-seller --once --max-steps 3 --json`
and then emits the same replay evidence files so operator review has a stable
artifact shape.

## Real Account Follow-Up

Real Facebook Marketplace credentials are intentionally out of scope for this
repo. The operator follow-up is:

1. Create a dedicated test seller account and document the allowed marketplace.
2. Store credentials in the sandbox secret manager, not in this repository.
3. Replace the `.invalid` sandbox URLs in `e2b_recording_fixture.json` with the
   approved test-account capture.
4. Re-record the six evidence frames after every major Marketplace UI change.

The worker must never publish a live listing without an explicit operator
approval on the `post_listing` action.
