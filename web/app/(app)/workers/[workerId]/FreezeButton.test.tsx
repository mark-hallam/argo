import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    replace: vi.fn(),
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
}));

const freezeMock = vi.fn();
const unfreezeMock = vi.fn();
vi.mock("@/lib/api/workers", () => ({
  freezeWorker: (...args: unknown[]) => freezeMock(...args),
  unfreezeWorker: (...args: unknown[]) => unfreezeMock(...args),
}));

import { FreezeButton } from "./FreezeButton";

const WORKER_ID = "018f67d0-0000-7000-8000-000000000002";

beforeEach(() => {
  freezeMock.mockReset();
  unfreezeMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("FreezeButton", () => {
  it("renders a red Freeze worker button when not frozen", () => {
    render(<FreezeButton workerId={WORKER_ID} isFrozen={false} />);
    const button = screen.getByTestId("worker-freeze-button");
    expect(button).toBeInTheDocument();
    expect(button).toHaveTextContent("Freeze worker");
    expect(button.className).toMatch(/destructive/);
  });

  it("enables the outer Freeze worker button when can_freeze is true", () => {
    render(
      <FreezeButton
        workerId={WORKER_ID}
        isFrozen={false}
        permissions={{ can_freeze: true }}
      />,
    );

    expect(screen.getByTestId("worker-freeze-button")).not.toBeDisabled();
  });

  it("disables the outer Freeze worker button when can_freeze is false", () => {
    render(
      <FreezeButton
        workerId={WORKER_ID}
        isFrozen={false}
        permissions={{ can_freeze: false }}
      />,
    );

    const button = screen.getByTestId("worker-freeze-button");
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute(
      "aria-label",
      "You don't have permission to freeze this worker",
    );
    expect(button).toHaveAttribute(
      "title",
      "You don't have permission to freeze this worker",
    );
  });

  it("clicking opens the confirm modal", () => {
    render(<FreezeButton workerId={WORKER_ID} isFrozen={false} />);
    expect(screen.queryByTestId("worker-freeze-dialog")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("worker-freeze-button"));
    expect(screen.getByTestId("worker-freeze-dialog")).toBeInTheDocument();
  });

  it("keeps the confirm button disabled when the typed id is wrong", () => {
    render(<FreezeButton workerId={WORKER_ID} isFrozen={false} />);
    fireEvent.click(screen.getByTestId("worker-freeze-button"));

    fireEvent.change(screen.getByTestId("worker-freeze-confirm-id"), {
      target: { value: "wrong-id" },
    });
    fireEvent.change(screen.getByTestId("worker-freeze-reason"), {
      target: { value: "investigating prompt injection" },
    });

    expect(screen.getByTestId("worker-freeze-confirm")).toBeDisabled();
  });

  it("enables the confirm button only when both id and reason are valid", async () => {
    freezeMock.mockResolvedValue({
      worker_id: WORKER_ID,
      frozen_at: "2026-05-20T10:00:00Z",
      frozen_by: "alice@acme",
      reason: "investigating",
    });
    render(<FreezeButton workerId={WORKER_ID} isFrozen={false} />);
    fireEvent.click(screen.getByTestId("worker-freeze-button"));

    const confirmBtn = screen.getByTestId("worker-freeze-confirm");
    expect(confirmBtn).toBeDisabled();

    fireEvent.change(screen.getByTestId("worker-freeze-confirm-id"), {
      target: { value: WORKER_ID },
    });
    expect(confirmBtn).toBeDisabled();

    fireEvent.change(screen.getByTestId("worker-freeze-reason"), {
      target: { value: "investigating prompt injection" },
    });
    expect(confirmBtn).not.toBeDisabled();

    fireEvent.click(confirmBtn);
    await waitFor(() =>
      expect(freezeMock).toHaveBeenCalledWith(
        WORKER_ID,
        "investigating prompt injection",
      ),
    );
  });

  it("when isFrozen=true, flips to Unfreeze and bypasses name-typing", async () => {
    unfreezeMock.mockResolvedValue({
      worker_id: WORKER_ID,
      unfrozen: true,
      unfrozen_by: "alice@acme",
    });
    render(<FreezeButton workerId={WORKER_ID} isFrozen={true} />);

    const unfreezeBtn = screen.getByTestId("worker-unfreeze-button");
    expect(unfreezeBtn).toBeInTheDocument();
    expect(unfreezeBtn).toHaveTextContent("Unfreeze worker");
    expect(screen.queryByTestId("worker-freeze-button")).not.toBeInTheDocument();

    fireEvent.click(unfreezeBtn);
    await waitFor(() => expect(unfreezeMock).toHaveBeenCalledWith(WORKER_ID));
    expect(screen.queryByTestId("worker-freeze-dialog")).not.toBeInTheDocument();
  });
});
