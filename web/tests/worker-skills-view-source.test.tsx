import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { SkillsActiveCard } from "@/components/skycot/worker/SkillsActiveCard";
import type { ActiveSkill } from "@/lib/api/worker-skills";

const WORKER_ID = "018f67d0-0000-7000-8000-000000000002";

const skill: ActiveSkill = {
  id: "skill-abc",
  name: "lead-qualifier",
  version: "1.4.2",
  trustScore: 92,
  usageCount7d: 17,
  learnedAt: null,
};

describe("Worker skills — View source", () => {
  it("renders View source as a link to /skills/<name>, not a toast-stub button", () => {
    render(
      <SkillsActiveCard
        workerId={WORKER_ID}
        workerName="Mariah"
        skills={[skill]}
        onDisable={() => {}}
      />,
    );

    const link = screen.getByTestId(`skill-view-${skill.id}`);
    expect(link.tagName).toBe("A");
    expect(link).toHaveAttribute("href", "/skills/lead-qualifier");
    expect(link).toHaveTextContent("View source");
  });

  it("URL-encodes skill names with reserved characters", () => {
    render(
      <SkillsActiveCard
        workerId={WORKER_ID}
        workerName="Mariah"
        skills={[{ ...skill, name: "lead qualifier/v2" }]}
        onDisable={() => {}}
      />,
    );

    const link = screen.getByTestId(`skill-view-${skill.id}`);
    expect(link).toHaveAttribute("href", "/skills/lead%20qualifier%2Fv2");
  });

  it("renders no announcement toast — View source navigates instead of stubbing", () => {
    render(
      <SkillsActiveCard
        workerId={WORKER_ID}
        workerName="Mariah"
        skills={[skill]}
        onDisable={() => {}}
      />,
    );

    expect(screen.queryByTestId("skills-toast")).not.toBeInTheDocument();
    expect(screen.queryByTestId("skills-announcement")).not.toBeInTheDocument();
  });
});
