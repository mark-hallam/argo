import { createElement, type ReactNode } from "react";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import WorkspaceSettingsPage from "@/app/(app)/settings/workspace/page";
import { ThemeProvider } from "@/components/skycot/ThemeProvider";

function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (key) => (data.has(key) ? (data.get(key) as string) : null),
    key: (index) => Array.from(data.keys())[index] ?? null,
    removeItem: (key) => {
      data.delete(key);
    },
    setItem: (key, value) => {
      data.set(key, String(value));
    },
  };
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
  return storage;
}

function renderWithTheme(children: ReactNode) {
  return render(createElement(ThemeProvider, null, children));
}

function renderWorkspaceSettings() {
  return renderWithTheme(createElement(WorkspaceSettingsPage));
}

describe("settings workspace theme picker", () => {
  beforeEach(() => {
    installMemoryStorage();
    window.localStorage.clear();
    document.documentElement.className = "";
    document.documentElement.removeAttribute("data-theme");
  });

  afterEach(() => {
    cleanup();
  });

  it("renders the theme picker in workspace settings and persists color mode", async () => {
    const user = userEvent.setup();
    const firstRender = renderWorkspaceSettings();

    expect(screen.getByTestId("settings-workspace-theme-picker")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Light" }));

    await waitFor(() => {
      expect(window.localStorage.getItem("theme")).toBe("light");
      expect(document.documentElement.classList.contains("light")).toBe(true);
      expect(document.documentElement.getAttribute("data-theme")).toBe("skycot-light");
    });

    firstRender.unmount();
    renderWorkspaceSettings();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute("aria-pressed", "true");
      expect(window.localStorage.getItem("theme")).toBe("light");
      expect(document.documentElement.getAttribute("data-theme")).toBe("skycot-light");
    });
  });

  it("persists the tenant theme selection across remounts", async () => {
    const user = userEvent.setup();
    const firstRender = renderWorkspaceSettings();

    await user.selectOptions(screen.getByLabelText("Tenant theme"), "acme");

    await waitFor(() => {
      expect(window.localStorage.getItem("skycot.tenantTheme")).toBe("acme");
      expect(document.documentElement.getAttribute("data-theme")).toBe("acme-light");
    });

    firstRender.unmount();
    renderWorkspaceSettings();

    await waitFor(() => {
      expect(screen.getByLabelText("Tenant theme")).toHaveValue("acme");
      expect(document.documentElement.getAttribute("data-theme")).toBe("acme-light");
    });
  });
});
