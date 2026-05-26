import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ActivityRow, formatHashForDisplay } from "@/components/skycot/ActivityRow";
import { SAMPLE_AUDIT_ROWS } from "@/lib/sample-audit";
import type { WorkerActivity } from "@/lib/api/worker-activity";

vi.mock("next/link", () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: React.ReactNode }) => (
    <a {...props}>{children}</a>
  ),
}));

const successMock = vi.fn();

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: successMock,
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

const writeText = vi.fn(async (_text: string) => undefined);

function installClipboardSpy() {
  Object.defineProperty(globalThis.navigator, "clipboard", {
    configurable: true,
    writable: true,
    value: { writeText },
  });
}

installClipboardSpy();

beforeEach(() => {
  writeText.mockClear();
  successMock.mockClear();
  installClipboardSpy();
});

afterEach(() => {
  vi.clearAllMocks();
});

const FULL_HASH = "abc1234567890def1234567890abcdef1234567890abcdef1234567890abcdef";

function makeActivity(overrides: Partial<WorkerActivity> = {}): WorkerActivity {
  return {
    id: "act_test_001",
    workerId: "wkr_test",
    workerName: "Test Worker",
    workerInitials: "TW",
    timestamp: "2026-05-21T12:00:00.000Z",
    verb: "did the thing",
    actionType: "message_sent",
    status: "succeeded",
    hash: FULL_HASH,
    ...overrides,
  };
}

describe("ActivityRow — hash cell", () => {
  it("renders a truncated Etherscan-style hash when activity.hash is present", () => {
    render(<ActivityRow activity={makeActivity()} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    expect(cell).toBeInTheDocument();
    expect(cell.textContent).toContain("0xabc1");
    expect(cell.textContent).toContain("cdef");
    expect(cell.textContent).toContain("…");
  });

  it("omits the hash cell entirely when activity.hash is undefined", () => {
    render(<ActivityRow activity={makeActivity({ hash: undefined })} />);
    expect(
      screen.queryByTestId("activity-row-hash-act_test_001"),
    ).not.toBeInTheDocument();
  });

  it("attaches an aria-label that surfaces the full hash for assistive tech", () => {
    render(<ActivityRow activity={makeActivity()} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    expect(cell.getAttribute("aria-label")).toBe(
      `Self-hash ${FULL_HASH}, click to copy`,
    );
  });

  it("exposes the full hash via title for hover tooltips before copy", () => {
    render(<ActivityRow activity={makeActivity()} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    expect(cell.getAttribute("title")).toContain(FULL_HASH);
    expect(cell.getAttribute("title")).toContain("Self-hash · click to copy");
  });

  it("labels sample hashes as non-production sample data", () => {
    render(<ActivityRow activity={makeActivity({ hashKind: "sample" })} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    expect(cell.getAttribute("data-hash-kind")).toBe("sample");
    expect(cell.getAttribute("title")).toContain(
      "Sample row — not part of the production audit chain",
    );
  });

  it("copies the FULL hash (not the truncated display) on click and fires a toast", async () => {
    render(<ActivityRow activity={makeActivity()} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    await act(async () => {
      fireEvent.click(cell);
      // Microtask drain so the async clipboard promise settles
      // before the assert and the post-resolve setState lands.
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith(FULL_HASH);
    expect(successMock).toHaveBeenCalledTimes(1);
    expect(successMock).toHaveBeenCalledWith(
      "Hash copied",
      expect.objectContaining({ description: FULL_HASH }),
    );
  });

  it("copies the full sample hash on click and fires a sample toast", async () => {
    render(<ActivityRow activity={makeActivity({ hashKind: "sample" })} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    await act(async () => {
      fireEvent.click(cell);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith(FULL_HASH);
    expect(successMock).toHaveBeenCalledWith(
      "Sample hash copied",
      expect.objectContaining({ description: FULL_HASH }),
    );
  });

  it("does not navigate when the hash cell is clicked", async () => {
    render(<ActivityRow activity={makeActivity()} />);
    const cell = screen.getByTestId("activity-row-hash-act_test_001");
    const event = new MouseEvent("click", { bubbles: true, cancelable: true });
    await act(async () => {
      cell.dispatchEvent(event);
      await Promise.resolve();
    });
    expect(event.defaultPrevented).toBe(true);
  });
});

describe("formatHashForDisplay", () => {
  it("formats a long hex string as 0xfirst4…last4", () => {
    expect(formatHashForDisplay("abc1234567890def")).toBe("0xabc1…0def");
  });

  it("keeps short hashes unchanged so test fixtures stay readable", () => {
    expect(formatHashForDisplay("abc1")).toBe("0xabc1");
  });

  it("does not double-prefix when input already starts with 0x", () => {
    expect(formatHashForDisplay("0xabc1234567890def")).toBe("0xabc1…0def");
  });
});

describe("sample-audit seed hashes (deterministic)", () => {
  it("populates a mock-only 64-char hex sampleHash on every seed row", () => {
    expect(SAMPLE_AUDIT_ROWS.length).toBeGreaterThan(0);
    for (const row of SAMPLE_AUDIT_ROWS) {
      expect(row.hash).toBeUndefined();
      expect(row.sampleHash).toBeDefined();
      expect(row.sampleHash).toMatch(/^[0-9a-f]{64}$/);
    }
  });

  it("produces distinct sample hashes for distinct rows", () => {
    const seen = new Set<string>();
    for (const row of SAMPLE_AUDIT_ROWS) {
      seen.add(row.sampleHash!);
    }
    expect(seen.size).toBe(SAMPLE_AUDIT_ROWS.length);
  });
});
