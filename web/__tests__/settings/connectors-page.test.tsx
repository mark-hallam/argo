import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, fireEvent } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/api/connectors", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/connectors")>(
    "@/lib/api/connectors",
  );
  return {
    ...actual,
    listTenantConnectors: vi.fn(),
    disconnectTenantConnector: vi.fn(),
    saveTenantConnectorCredential: vi.fn(),
  };
});

import ConnectorsPage from "@/app/(app)/settings/connectors/page";
import * as api from "@/lib/api/connectors";

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  vi.mocked(api.listTenantConnectors).mockResolvedValue({ connectors: [] });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/settings/connectors page", () => {
  it("renders the hero, search bar, and 29 connector cards", async () => {
    render(wrap(<ConnectorsPage />));

    expect(await screen.findByTestId("connectors-page")).toBeInTheDocument();
    expect(screen.getByTestId("connectors-search")).toBeInTheDocument();
    expect(screen.getByTestId("connectors-count")).toHaveTextContent("29 total");

    expect(screen.getByTestId("connector-card-notion")).toBeInTheDocument();
    expect(screen.getByTestId("connector-card-slack")).toBeInTheDocument();
    expect(screen.getByTestId("connector-card-stripe-events")).toBeInTheDocument();
  });

  it("narrows the grid when the operator searches", async () => {
    render(wrap(<ConnectorsPage />));

    fireEvent.change(screen.getByTestId("connectors-search"), {
      target: { value: "stripe" },
    });

    expect(screen.getByTestId("connector-card-stripe")).toBeInTheDocument();
    expect(screen.queryByTestId("connector-card-notion")).toBeNull();
  });

  it("shows the empty-filter message when nothing matches", async () => {
    render(wrap(<ConnectorsPage />));

    fireEvent.change(screen.getByTestId("connectors-search"), {
      target: { value: "no-such-connector-zzz" },
    });

    expect(screen.getByTestId("connectors-empty-filter")).toBeInTheDocument();
  });

  it("opens the connect modal on the Notion (api-key) card", async () => {
    render(wrap(<ConnectorsPage />));

    fireEvent.click(screen.getByTestId("connector-connect-notion"));

    expect(await screen.findByTestId("connector-modal")).toHaveAttribute(
      "data-connector-id",
      "notion",
    );
    expect(screen.getByTestId("connector-modal-apikey-form")).toBeInTheDocument();
  });

  it("opens the connect modal on the Gmail (oauth) card with the OAuth-start link", async () => {
    render(wrap(<ConnectorsPage />));

    fireEvent.click(screen.getByTestId("connector-connect-gmail"));

    expect(await screen.findByTestId("connector-modal")).toHaveAttribute(
      "data-connector-id",
      "gmail",
    );
    const link = screen.getByTestId("connector-modal-oauth-start") as HTMLAnchorElement;
    expect(link).toBeInTheDocument();
    expect(link.href).toMatch(/\/connect\/gmail\?/);
  });
});
