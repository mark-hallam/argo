---
name: claim4_price_check
version: 0.1.0
description: "Published Claim-4 fixture for checking marketplace listing prices"
capabilities: []
provenance:
  source: published:verified:claim4-marketplace
  hash: sha256-claim4-fixture
lock: none
evals:
  - input:
      listing: claim4-demo
      expected_price: USD 42
    expected:
      kind: contains
      text: USD 42
---

# claim4_price_check

Use the seller dashboard search to locate a listing, compare its visible price
to the requested price, and return the listing URL plus a short confirmation.

This fixture represents the marketplace-reloaded copy of a skill that was first
learned from a computer-use trajectory, reviewed, evaluated, and published.
