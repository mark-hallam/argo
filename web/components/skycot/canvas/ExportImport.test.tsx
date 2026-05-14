import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ExportImport } from "./ExportImport";
import type { SkillIrBundle } from "@/lib/api/skill-ir-export";
import type { SkillGraph } from "@/lib/types/skill-ir";

function graph(label = "Start"): SkillGraph {
  return {
    entry: "a",
    nodes: [
      {
        id: "a",
        label,
        kind: { type: "action", instruction: "start" },
        annotations: {},
      },
    ],
    edges: [],
    variables: [],
    metadata: {},
  };
}

function bundle(overrides: Partial<SkillIrBundle> = {}): SkillIrBundle {
  return {
    schemaVersion: "skycot.skill_ir.bundle.v1",
    sourceSkillId: "source-skill",
    version: "v-import",
    irHash: "hash-import",
    graph: graph("Imported"),
    ...overrides,
  };
}

describe("ExportImport", () => {
  it("exports the current graph as a downloaded bundle", async () => {
    const user = userEvent.setup();
    const createObjectURL = vi.fn().mockReturnValue("blob:test");
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: createObjectURL });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revokeObjectURL });
    const exportBundle = vi.fn().mockResolvedValue(bundle());
    render(
      <ExportImport
        skillId="skill-1"
        graph={graph()}
        version="v1"
        exportBundle={exportBundle}
      />,
    );

    await user.click(screen.getByTestId("export-skill-ir"));

    expect(exportBundle).toHaveBeenCalledWith("skill-1", "v1", graph(), undefined);
    expect(createObjectURL).toHaveBeenCalledOnce();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:test");
  });

  it("surfaces invalid import files inline", async () => {
    const user = userEvent.setup();
    render(<ExportImport skillId="skill-1" graph={graph()} version="v1" />);

    const file = new File([JSON.stringify({ schemaVersion: "old" })], "bad.skill.json", {
      type: "application/json",
    });
    await user.upload(screen.getByTestId("skill-ir-import-file"), file);

    expect(screen.getByTestId("skill-ir-import-error")).toHaveTextContent(
      /Unsupported skill bundle schema/,
    );
  });

  it("previews and adopts a valid import bundle", async () => {
    const user = userEvent.setup();
    const importBundle = vi.fn().mockResolvedValue({
      skillId: "skill-1",
      version: "canvas-2",
      irHash: "hash-import",
      importedHash: "hash-import",
      graph: graph("Imported"),
      abTestScheduled: true,
      abTestRunId: "run-1",
    });
    const onAdopt = vi.fn();
    render(
      <ExportImport
        skillId="skill-1"
        graph={graph()}
        version="v1"
        importBundle={importBundle}
        onAdopt={onAdopt}
      />,
    );

    const file = new File([JSON.stringify(bundle())], "ok.skill.json", {
      type: "application/json",
    });
    await user.upload(screen.getByTestId("skill-ir-import-file"), file);
    expect(screen.getByTestId("skill-ir-import-preview")).toBeInTheDocument();

    await user.click(screen.getByTestId("adopt-skill-ir-import"));

    expect(importBundle).toHaveBeenCalledWith("skill-1", bundle());
    expect(onAdopt).toHaveBeenCalledOnce();
    expect(await screen.findByTestId("skill-ir-import-success")).toHaveTextContent(
      /canvas-2/,
    );
  });
});
