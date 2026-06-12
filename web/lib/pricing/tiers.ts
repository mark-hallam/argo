// Typed projection of the canonical pricing source of truth
// (docs/skycot/pricing/v1-tiers.toml → tiers.data.json, DEC-035 / DEC-053).
//
// Every number on /pricing — prices, action allowances, the overage
// multiplier, and the default spend cap — is read from here. There are no
// hardcoded prices in the page JSX; `web/tests/pricing-tiers-source.test.ts`
// pins this module's source (the JSON) to the .toml.
import data from "./tiers.data.json";

export interface FreeTierNumbers {
  price_monthly_usd: number;
  max_workers: number;
  max_actions_per_month: number;
}

export interface ProTierNumbers extends FreeTierNumbers {
  /** Per-teammate monthly action allowance bundled in the $49 (DEC-053). */
  included_actions: number;
  /** Metered overage rate = token cost × this multiplier (DEC-053). */
  overage_multiplier: number;
  /** Customer-visible default hard spend cap, pre-seeded on /billing. */
  default_monthly_cap_usd: number;
}

export interface TiersData {
  free: FreeTierNumbers;
  pro: ProTierNumbers;
  enterprise: FreeTierNumbers;
}

export const TIERS_DATA = data as unknown as TiersData;

/** "$0", "$49" — whole-dollar tier price label. */
export function usd(amount: number): string {
  return `$${amount.toLocaleString("en-US")}`;
}

/** "1,500", "5,000" — grouped integer for action counts. */
export function actions(count: number): string {
  return count.toLocaleString("en-US");
}

export const FREE = TIERS_DATA.free;
export const PRO = TIERS_DATA.pro;

/** "× 1.4" multiplier label for the metered-overage copy. */
export const OVERAGE_MULTIPLIER_LABEL = `× ${PRO.overage_multiplier}`;

/**
 * Worked overage example for the FAQ. Token cost is the variable the customer
 * controls, so the example is expressed against an illustrative token cost
 * rather than a fixed per-action price (overage is token cost × multiplier,
 * not a flat fee). The cap is the honest "usage stops, no surprise bill" line.
 */
export function overageExample(illustrativeTokenCostUsd = 10): string {
  const charged = (illustrativeTokenCostUsd * PRO.overage_multiplier).toFixed(0);
  return (
    `Your $49 includes ${actions(PRO.included_actions)} actions per teammate each month. ` +
    `Past that, overage bills at your token cost ${OVERAGE_MULTIPLIER_LABEL} — ` +
    `so if the extra actions cost $${illustrativeTokenCostUsd} in model tokens, you pay $${charged}. ` +
    `Usage stops at your spend cap (default ${usd(PRO.default_monthly_cap_usd)}, you set it), so there's never a surprise bill.`
  );
}
