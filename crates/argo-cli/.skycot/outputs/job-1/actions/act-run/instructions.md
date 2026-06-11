# Post listing — `(unset)` on `(unset)`

Approved by `gateway` at `2026-06-11T00:47:34.134372+00:00`.

Title:    `(unset)`
Price:    `$(unset)`
Item:     `(unset)`

Steps:
1. Sign into your `(unset)` seller account.
2. Create a new listing with the title above.
3. Paste the description from the listing draft (see `assistant_output.md`).
4. Set price to `$(unset)` and shipping per the draft.
5. Upload photos (see `instructions.md` for the photo list if attached).
6. Publish.

When live, copy the listing URL and run:

```
argo worker actions complete act-run --note "live" \
  --result-json '{"listing_url": "<url>", "platform": "(unset)"}'
```
