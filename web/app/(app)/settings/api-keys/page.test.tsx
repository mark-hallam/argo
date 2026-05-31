import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const hooks = vi.hoisted(() => ({
  useApiKeys: vi.fn(),
  useCreateApiKey: vi.fn(),
  useRevokeApiKey: vi.fn(),
}));

vi.mock("@/lib/hooks/use-api-keys", () => hooks);
vi.mock("./styles", () => ({
  ApiKeysStyles: () => null,
}));

import ApiKeysPage from "./page";

describe("settings api-keys page", () => {
  beforeEach(() => {
    hooks.useApiKeys.mockReturnValue({
      data: { keys: [], source: "mock-bypass" },
      isLoading: false,
      isError: false,
      error: null,
    });
    hooks.useCreateApiKey.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
      error: null,
    });
    hooks.useRevokeApiKey.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("shows an honest unavailable state when the live API-key endpoint errors", () => {
    hooks.useApiKeys.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: new Error("404: no api-keys endpoint"),
    });

    render(<ApiKeysPage />);

    expect(screen.getByText("API-key storage is not live.")).toBeInTheDocument();
    expect(screen.getByText(/No keys are being mocked in demo-live mode/)).toBeInTheDocument();
    expect(screen.getByText("404: no api-keys endpoint")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /create api key/i })).toBeDisabled();
    expect(screen.queryByText("No API keys yet.")).not.toBeInTheDocument();
  });
});
