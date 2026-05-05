# Approval checklist

- stop_reason: `approval_required`

---

## approval_checklist

### Safe to approve

- [ ] **Listing draft (Facebook Marketplace)** — title, description,
      photos, price, location all sourced from operator brief +
      verified comp listings.
      - sources: <https://www.facebook.com/marketplace/item/0001> (comp $145, sold last week),
        <https://www.facebook.com/marketplace/item/0002> (comp $160 active)
- [ ] **Listing draft (eBay)** — same listing reformatted for eBay's
      item-specifics requirements. Pricing aligned with FB.
      - sources: <https://www.ebay.com/itm/00001> (sold $155),
        <https://www.ebay.com/itm/00002> (sold $158)
- [ ] **Verify-price action** — price target $150 reserve sits at the
      median of comps ($150–$155). Within band. Run autonomously
      via `argo worker actions execute <verify_price_id>`.

### Needs human verification

- [ ] **Photo coverage** — drafts assume 6 photos (the operator's
      brief said "perfect condition, original box"). The
      operator must take + upload the actual photos before
      clicking publish on each platform.
- [ ] **Shipping cost** — drafts quote a flat $25 shipping
      derived from "size 12 inline skates" weight estimates.
      Operator should re-quote against actual shipping carrier
      rates if an offer comes in.

### Blocked

- _(none — research evidence sufficient: 6 web_search hits + 4 web_extract pages totalling 8200 chars)_
