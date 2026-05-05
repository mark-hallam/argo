# Marketplace seller — sample operator prompts

8 prompts that produce good worker output against the
`gtm-engineer` vertical. Use these as templates for your own
items.

The pattern that works:

1. **Item identity** — brand, model, year, condition. Specific
   beats vague (`Powerslide Imperial 110mm` > `pair of skates`).
2. **Where you want to sell** — name the platform(s). The
   worker uses this to pick per-platform instruction templates
   (#732).
3. **A reserve / floor price** — gives the worker something to
   `verify_price` against. Without it, the worker has to guess
   "what's the operator willing to take?".
4. **Anything unusual** — defects, missing accessories,
   urgency. Workers can't see what you can't tell them.

---

## skates

```
Sell my Powerslide Imperial 110mm inline skates, size 12 US,
used 8 times, perfect condition, original box. List on Facebook
Marketplace AND eBay. Reserve $150.
```

Expected: 1× verify_price (autonomous), 2× post_listing
(approval-gated, one per platform).

## hot-tub-cover

```
Sell my Caldera Spas hot tub cover, fits 88x88" tub, 3 years
old, no rips but mild fading. Local pickup only. Facebook
Marketplace only. Reserve $80, will accept $50.
```

Expected: 1× verify_price (autonomous), 1× post_listing
(approval-gated).

## road-bike

```
Sell my 2021 Trek Domane SL 7 in size 56, less than 500 miles,
upgraded Shimano Ultegra Di2. eBay national + Facebook
Marketplace local. Reserve $4500.
```

Expected: research-heavy (high-value item), evidence-thin
worker runs may auto-block the post_listing actions.

## kids-toy

```
Sell our toddler's outgrown Cozy Coupe car. Faded, all 4 wheels
intact, no horn. Local only. Facebook Marketplace. $20 OBO.
```

Expected: low-research; verify_price might autonomously verify
against $20 (well below comp range so no concerns).

## ambiguous-item

```
Sell my old laptop. Apple. Pretty old.
```

Expected: worker should stop with `[STOP: NEED_INFO]` —
"old Apple laptop" is too ambiguous for comps. The
`approval_checklist.md` should flag missing model + year.

## item-with-defect

```
Sell my Yamaha P-125 digital piano, 2020, fully working but the
sustain pedal is broken (works without it; pedal sold
separately on Yamaha for $40). eBay only. Reserve $400.
```

Expected: listings should mention the defect AND the
replacement-pedal context.

## reply-to-buyer

```
A buyer messaged me on Facebook about the skates listed
yesterday, asking if I'll take $130 cash + same-day pickup.
Reserve was $150. Should I accept?
```

Expected: the worker can `verify_price` (research) and emit a
`reply_to_buyer` (approval-gated, HIGH RISK) with a draft
response. Operator approves before the message goes out.

## bulk-relist

```
The skates I listed last week on eBay didn't sell. Drop the
price 10% and re-list.
```

Expected: 1× update_price (approval-gated) + 1× relist_item
(approval-gated). Reads memory for the previous listing's URL.
