import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  TrustRatchetCard,
  dismissKeyFor,
  isDismissed,
} from "./TrustRatchetCard";

// happy-dom doesn't expose a Storage implementation by default; mirror
// the in-memory shim used by CelebrationBanner.test so localStorage is
// available across this suite.
function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() { return data.size; },
    clear: () => data.clear(),
    getItem: (k) => (data.has(k) ? (data.get(k) as string) : null),
    key: (i) => Array.from(data.keys())[i] ?? null,
    removeItem: (k) => { data.delete(k); },
    setItem: (k, v) => { data.set(k, String(v)); },
  };
  Object.defineProperty(window, "localStorage", { configurable: true, value: storage });
  return storage;
}

const WORKER = { id: "worker-1", name: "Riley" };

function renderCard(
  overrides: Partial<React.ComponentProps<typeof TrustRatchetCard>> = {},
) {
  const onPromote = vi.fn();
  render(
    <TrustRatchetCard
      worker={WORKER}
      tool="update_roadmap"
      targetGlob="Q3-*"
      streakCount={10}
      recentApprovalsIso={[
        "2026-05-17T10:00:00Z",
        "2026-05-18T10:00:00Z",
        "2026-05-19T10:00:00Z",
      ]}
      onPromote={onPromote}
      dismissKeyPrefix="test-dismiss"
      {...overrides}
    />,
  );
  return { onPromote };
}

beforeEach(() => {
  installMemoryStorage();
  vi.restoreAllMocks();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("TrustRatchetCard", () => {
  it("renders the load-bearing structure", () => {
    renderCard();
    expect(screen.getByText(/You've approved/)).toBeInTheDocument();
    expect(screen.getByText(/Riley's update_roadmap/)).toBeInTheDocument();
    expect(screen.getByText(/10 times in a row/)).toBeInTheDocument();
    expect(screen.getByText(/for Q3-\*/)).toBeInTheDocument();
    expect(screen.getByTestId("trust-ratchet-promote")).toBeInTheDocument();
    expect(screen.getByTestId("trust-ratchet-dismiss")).toBeInTheDocument();
    expect(screen.getByTestId("trust-ratchet-recent")).toBeInTheDocument();
  });

  it("calls onPromote when the operator clicks", async () => {
    const user = userEvent.setup();
    const { onPromote } = renderCard();
    await user.click(screen.getByTestId("trust-ratchet-promote"));
    expect(onPromote).toHaveBeenCalledTimes(1);
  });

  it("dismiss hides the card and writes a 7-day expiry", async () => {
    const user = userEvent.setup();
    renderCard();
    await user.click(screen.getByTestId("trust-ratchet-dismiss"));
    expect(screen.queryByTestId("trust-ratchet-card")).not.toBeInTheDocument();
    const key = dismissKeyFor(
      "test-dismiss",
      { id: "worker-1" },
      "update_roadmap",
      "Q3-*",
    );
    const raw = window.localStorage.getItem(key);
    expect(raw).not.toBeNull();
    const expires = Number.parseInt(raw ?? "0", 10);
    expect(expires).toBeGreaterThan(Date.now());
    // 7 days +/- a tolerance for clock + JIT.
    expect(expires).toBeLessThan(Date.now() + 7 * 24 * 60 * 60 * 1000 + 5000);
  });

  it("stays hidden on re-render when an unexpired dismiss key exists", async () => {
    const key = dismissKeyFor(
      "test-dismiss",
      { id: "worker-1" },
      "update_roadmap",
      "Q3-*",
    );
    window.localStorage.setItem(
      key,
      String(Date.now() + 60 * 60 * 1000),
    );
    renderCard();
    // The dismiss effect runs synchronously enough; React Testing
    // Library waits a microtask. Use findBy* to give the effect a
    // tick, then assert absence.
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByTestId("trust-ratchet-card")).not.toBeInTheDocument();
  });

  it("dismiss expires after 7 days (isDismissed reflects TTL)", () => {
    const key = dismissKeyFor(
      "test-dismiss",
      { id: "worker-1" },
      "update_roadmap",
      "Q3-*",
    );
    // Set an expired marker (now - 1ms). isDismissed must report false.
    window.localStorage.setItem(key, String(Date.now() - 1));
    const stillDismissed = isDismissed(
      "test-dismiss",
      { id: "worker-1" },
      "update_roadmap",
      "Q3-*",
    );
    expect(stillDismissed).toBe(false);
  });

  it("renders without target_glob and without recent timestamps", () => {
    renderCard({ targetGlob: undefined, recentApprovalsIso: undefined });
    expect(screen.getByTestId("trust-ratchet-card")).toBeInTheDocument();
    expect(screen.queryByTestId("trust-ratchet-recent")).not.toBeInTheDocument();
    expect(screen.queryByText(/for Q3-\*/)).not.toBeInTheDocument();
  });
});
