/**
 * Semantic icon assignment for skill cards on the marketplace grid.
 *
 * The previous implementation rotated a 5-icon array by index modulo, so a
 * `pricing.competitor_scrape` skill could render a Sparkles icon and an
 * `invoice.extract` skill could render a Briefcase — meaningless to the
 * camera. The skill name prefix already carries the semantic cohort
 * (`pricing.*`, `invoice.*`, `candidate.*`), so map prefix → icon directly.
 */
import {
  ChartBar,
  Handshake,
  LifeBuoy,
  type LucideIcon,
  Receipt,
  ShieldCheck,
  Sparkles,
  Target,
  TrendingUp,
  Users,
} from "lucide-react";

const SKILL_ICONS: Record<string, LucideIcon> = {
  pricing: TrendingUp,
  invoice: Receipt,
  candidate: Users,
  lead: Target,
  approval: ShieldCheck,
  support: LifeBuoy,
  market: ChartBar,
  deal: Handshake,
};

export const DEFAULT_SKILL_ICON: LucideIcon = Sparkles;

/** Pick a semantic icon for a skill by its name prefix (`pricing.*` → TrendingUp). */
export function iconForSkill(name: string): LucideIcon {
  const prefix = (name.split(/[.:]/)[0] ?? "").toLowerCase();
  return SKILL_ICONS[prefix] ?? DEFAULT_SKILL_ICON;
}
