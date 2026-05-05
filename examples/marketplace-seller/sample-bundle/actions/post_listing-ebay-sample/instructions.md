# Post listing — `Powerslide Imperial 110 mm Inline Speed Skates — US Men's 12 — Used 8x` on **eBay**

Approved by `operator` at `2026-05-05T10:15:42Z`.

Listing:
- title:    `Powerslide Imperial 110 mm Inline Speed Skates — US Men's 12 — Used 8x`
- price:    `$160`
- item_id:  `skates-110`

eBay-specific checklist:
1. **Listing format** — pick `Buy it Now` for fixed-price OR `Auction` for high-demand items.
2. **Category** — let eBay auto-detect; override if it lands in the wrong sub-category.
3. **Item specifics** — fill EVERY required field (Brand, Model, Condition, Size). eBay's algo penalises sparse listings.
4. **Photos** — at least 4. eBay caps at 12 free.
5. **Shipping** — quote actual cost; never `local pickup only` unless the item ships heavy.
6. **Returns** — 30-day returns recommended; reduces buyer-side friction.

When live, copy the listing URL and run:

```
argo worker actions complete 019df8aa-1f12-7a01-9f10-2f4d77c8be21 --note "ebay live" \
  --result-json '{"listing_url": "<url>", "platform": "ebay"}'
```
