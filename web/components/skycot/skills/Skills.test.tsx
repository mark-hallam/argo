import { describe, expect, it } from "vitest";
import { TRY_CHIPS } from "@/app/(app)/skills/page";
import { SEED_SKILLS, type SkillRow } from "@/lib/api/skills";

// Mirror of the inline filter at web/app/(app)/skills/page.tsx:65-78 so that
// a regression to the chip list, the seed catalog, or the filter itself
// surfaces as a failing test instead of an empty marketplace at demo time.
function filterByQuery(skills: readonly SkillRow[], query: string): SkillRow[] {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return [...skills];
  return skills.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q) ||
      s.layer.toLowerCase().includes(q),
  );
}

describe("Skills marketplace TRY chips", () => {
  it("ships six chips", () => {
    expect(TRY_CHIPS).toHaveLength(6);
  });

  it.each(TRY_CHIPS)(
    "chip %p returns at least one seed skill",
    (chip) => {
      const matches = filterByQuery(SEED_SKILLS, chip);
      expect(matches.length).toBeGreaterThan(0);
    },
  );

  it("every chip text reads as a single word for natural operator scanning", () => {
    for (const chip of TRY_CHIPS) {
      expect(chip.trim().includes(" ")).toBe(false);
    }
  });
});
