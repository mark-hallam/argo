import { describe, expect, it } from "vitest";
import {
  ChartBar,
  Handshake,
  LifeBuoy,
  Receipt,
  ShieldCheck,
  Sparkles,
  Target,
  TrendingUp,
  Users,
} from "lucide-react";
import { DEFAULT_SKILL_ICON, iconForSkill } from "./skill-icons";

describe("lib/api/skill-icons", () => {
  it("maps each known seed-skill prefix to its semantic icon", () => {
    expect(iconForSkill("pricing.competitor_scrape")).toBe(TrendingUp);
    expect(iconForSkill("invoice.extract")).toBe(Receipt);
    expect(iconForSkill("candidate.screen")).toBe(Users);
    expect(iconForSkill("lead.enrich")).toBe(Target);
    expect(iconForSkill("approval.route")).toBe(ShieldCheck);
    expect(iconForSkill("support.triage")).toBe(LifeBuoy);
    expect(iconForSkill("deal.qualify")).toBe(Handshake);
    expect(iconForSkill("market.analysis")).toBe(ChartBar);
  });

  it("treats ':'-separated prefixes the same as '.'-separated", () => {
    expect(iconForSkill("approval:route")).toBe(ShieldCheck);
  });

  it("is case-insensitive on the prefix", () => {
    expect(iconForSkill("Pricing.scan")).toBe(TrendingUp);
    expect(iconForSkill("INVOICE.parse")).toBe(Receipt);
  });

  it("falls back to Sparkles for unknown prefixes", () => {
    expect(iconForSkill("custom.thing")).toBe(Sparkles);
    expect(iconForSkill("")).toBe(Sparkles);
    expect(DEFAULT_SKILL_ICON).toBe(Sparkles);
  });
});
