import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { InstallSkillButton } from "@/components/skycot/skills/InstallSkillButton";
import type { InstallSkillResult } from "@/lib/api/skills";

// The button drives the real `installSkillToWorkers` flow; we mock it so
// the component test asserts the verify → installing → installed state
// machine without a network round-trip. The real endpoint wiring is
// covered in `lib/api/skills.test.ts`.
vi.mock("@/lib/api/skills", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/skills")>("@/lib/api/skills");
  return { ...actual, installSkillToWorkers: vi.fn() };
});

import { installSkillToWorkers } from "@/lib/api/skills";

const mockedInstall = vi.mocked(installSkillToWorkers);
const SKILL_UUID = "11111111-1111-1111-1111-111111111111";

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
}

function renderButton(skillId = SKILL_UUID) {
  const client = makeQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <InstallSkillButton skillId={skillId} />
    </QueryClientProvider>,
  );
}

describe("InstallSkillButton", () => {
  beforeEach(() => {
    mockedInstall.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders an idle Install button by default", () => {
    renderButton();
    const btn = screen.getByTestId("skill-install-btn");
    expect(btn).toHaveTextContent("Install");
    expect(btn).toHaveAttribute("data-phase", "idle");
    expect(btn).not.toBeDisabled();
  });

  it("walks verify → installing → installed on a successful install", async () => {
    let resolveInstall: (r: InstallSkillResult) => void = () => {};
    mockedInstall.mockReturnValue(
      new Promise<InstallSkillResult>((resolve) => {
        resolveInstall = resolve;
      }),
    );

    const user = userEvent.setup();
    renderButton();
    const btn = screen.getByTestId("skill-install-btn");

    // Click → provenance-verify beat (the real POST has not fired yet).
    await user.click(btn);
    expect(btn).toHaveAttribute("data-phase", "verifying");
    expect(btn).toHaveTextContent("Verifying provenance…");
    expect(btn).toBeDisabled();
    expect(mockedInstall).not.toHaveBeenCalled();

    // After the verify pause the real assignment fires (installing).
    await waitFor(() => expect(mockedInstall).toHaveBeenCalledWith(SKILL_UUID));
    await waitFor(() =>
      expect(btn).toHaveAttribute("data-phase", "installing"),
    );
    expect(btn).toHaveTextContent("Installing skill…");

    // Resolve the install → terminal "Available to N workers".
    resolveInstall({
      skillId: SKILL_UUID,
      workerIds: ["w1", "w2", "w3"],
      source: "gateway",
    });
    await waitFor(() => expect(btn).toHaveAttribute("data-phase", "installed"));
    expect(btn).toHaveTextContent("Available to 3 workers");
  });

  it("surfaces a real error when the install is rejected", async () => {
    mockedInstall.mockRejectedValue(
      new Error("No workers to install this skill to — hire a teammate first."),
    );

    const user = userEvent.setup();
    renderButton();
    const btn = screen.getByTestId("skill-install-btn");

    await user.click(btn);
    await waitFor(() => expect(btn).toHaveAttribute("data-phase", "error"));
    expect(btn).toHaveTextContent("Install failed — retry");
    expect(screen.getByTestId("skill-install-error")).toHaveTextContent(
      "hire a teammate first",
    );
  });

  it("singularizes the worker count when only one worker is targeted", async () => {
    mockedInstall.mockResolvedValue({
      skillId: SKILL_UUID,
      workerIds: ["solo"],
      source: "gateway",
    });

    const user = userEvent.setup();
    renderButton();
    const btn = screen.getByTestId("skill-install-btn");

    await user.click(btn);
    await waitFor(() => expect(btn).toHaveAttribute("data-phase", "installed"));
    expect(btn).toHaveTextContent("Available to 1 worker");
  });
});
