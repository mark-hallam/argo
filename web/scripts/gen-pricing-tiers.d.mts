// Type declarations for the build-time pricing-tier generator (consumed by
// web/tests/pricing-tiers-source.test.ts for the drift pin).
export interface TierScalars {
  price_monthly_usd: number;
  max_workers: number;
  max_actions_per_month: number;
  included_actions?: number;
  overage_multiplier?: number;
  default_monthly_cap_usd?: number;
}

export interface TiersDataFile {
  _comment: string;
  free: TierScalars;
  pro: TierScalars;
  enterprise: TierScalars;
}

export function parseTierScalars(toml: string): Record<string, TierScalars>;
export function buildTiersData(toml: string): TiersDataFile;
