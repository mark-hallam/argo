import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CelebrationBanner, CelebrationBannerView } from "./CelebrationBanner";
import {
  CELEBRATIONS_STORAGE_KEY,
  setCelebration,
  type CelebrationEntry,
} from "@/lib/hire/celebration-storage";

vi.mock("@/lib/hooks/use-approvals", () => ({
  useApprovals: () => ({ data: [] }),
}));

vi.mock("next/link", () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => (data.has(k) ? (data.get(k) as string) : null),
    key: (i) => Array.from(data.keys())[i] ?? null,
    removeItem: (k) => {
      data.delete(k);
    },
    setItem: (k, v) => {
      data.set(k, String(v));
    },
  };
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
  return storage;
}

const ACTIVE: CelebrationEntry = {
  workerId: "wkr_01",
  workerName: "Mariah",
  hiredAt: new Date(Date.now() - 5_000).toISOString(),
  inert: false,
  ttl: new Date(Date.now() + 60_000).toISOString(),
};
const INERT: CelebrationEntry = {
  workerId: "wkr_02",
  workerName: "Priya",
  hiredAt: new Date().toISOString(),
  inert: true,
  ttl: new Date(Date.now() + 60_000).toISOString(),
};

beforeEach(() => {
  installMemoryStorage();
  vi.restoreAllMocks();
  vi.clearAllMocks();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("CelebrationBannerView (presentational)", () => {
  it("renders the active variant with the profile CTA", () => {
    render(
      <CelebrationBannerView entries={[ACTIVE]} onDismiss={() => {}} />,
    );
    const banner = screen.getByTestId("celebration-banner");
    expect(banner.getAttribute("data-variant")).toBe("active");
    expect(banner).toHaveTextContent(/Mariah is hired/);
    expect(banner).toHaveTextContent(/start working in the background/);
    const cta = screen.getByTestId("celebration-banner-cta");
    expect(cta.getAttribute("href")).toBe("/workers/wkr_01");
    expect(cta).toHaveTextContent(/Open Mariah's profile/);
  });

  it("renders the inert variant with the Connect tools CTA pointing at settings", () => {
    render(
      <CelebrationBannerView entries={[INERT]} onDismiss={() => {}} />,
    );
    const banner = screen.getByTestId("celebration-banner");
    expect(banner.getAttribute("data-variant")).toBe("inert");
    expect(banner).toHaveTextContent(/Priya is hired but needs tools/);
    const cta = screen.getByTestId("celebration-banner-cta");
    expect(cta.getAttribute("href")).toBe("/workers/wkr_02?tab=settings");
    expect(cta).toHaveTextContent(/Connect tools/);
  });

  it("renders nothing when there are no entries", () => {
    const { container } = render(
      <CelebrationBannerView entries={[]} onDismiss={() => {}} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("shows only the most-recent entry when multiple celebrations queue", () => {
    render(
      <CelebrationBannerView entries={[ACTIVE, INERT]} onDismiss={() => {}} />,
    );
    // Only one banner; the trailing (most-recent-by-hiredAt) entry wins.
    const banners = screen.getAllByTestId("celebration-banner");
    expect(banners).toHaveLength(1);
    expect(banners[0]).toHaveTextContent(/Priya/);
  });

  it("calls onDismiss with the visible worker's id when × clicked", async () => {
    const user = userEvent.setup();
    const dismiss = vi.fn();
    render(
      <CelebrationBannerView entries={[ACTIVE]} onDismiss={dismiss} />,
    );
    await user.click(screen.getByTestId("celebration-banner-dismiss"));
    expect(dismiss).toHaveBeenCalledWith("wkr_01");
  });
});

describe("CelebrationBanner (hook-backed)", () => {
  it("reads from localStorage on mount and renders the queued celebration", async () => {
    setCelebration("wkr_03", { workerName: "Jamal" });
    render(<CelebrationBanner />);
    // The hook reads localStorage in a useEffect; assertion waits for it.
    expect(await screen.findByTestId("celebration-banner")).toHaveTextContent(
      /Jamal is hired/,
    );
  });

  it("clicking × removes the entry from storage and unmounts the banner", async () => {
    const user = userEvent.setup();
    setCelebration("wkr_04", { workerName: "Ada" });
    render(<CelebrationBanner />);
    const dismiss = await screen.findByTestId("celebration-banner-dismiss");
    await user.click(dismiss);
    expect(screen.queryByTestId("celebration-banner")).toBeNull();
    const raw = window.localStorage.getItem(CELEBRATIONS_STORAGE_KEY);
    expect(raw === null || raw === "[]" || JSON.parse(raw) /* keep TS happy */).toBeTruthy();
    expect(raw === null ? [] : JSON.parse(raw)).toEqual([]);
  });

  it("expired entries are silently dropped before paint", () => {
    // Pre-seed an expired entry directly so the hook's pruneExpired runs.
    const expired: CelebrationEntry = {
      workerId: "wkr_old",
      workerName: "Forgotten",
      hiredAt: new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString(),
      inert: false,
      ttl: new Date(Date.now() - 60_000).toISOString(),
    };
    window.localStorage.setItem(
      CELEBRATIONS_STORAGE_KEY,
      JSON.stringify([expired]),
    );
    render(<CelebrationBanner />);
    expect(screen.queryByTestId("celebration-banner")).toBeNull();
  });
});

describe("CelebrationBanner — auto-clear on first approval", () => {
  it("clears the celebration when an approval with the matching worker name arrives", async () => {
    vi.resetModules();
    vi.doMock("@/lib/hooks/use-approvals", () => ({
      useApprovals: () => ({
        data: [{ id: "ap_1", worker: { name: "Mariah" }, verb: "x", detail: "y" }],
      }),
    }));
    const { CelebrationBanner: BannerWithApprovals } = await import("./CelebrationBanner");
    setCelebration("wkr_match", { workerName: "Mariah" });
    render(<BannerWithApprovals />);
    // Auto-clear runs in a follow-up effect; banner should be absent after settle.
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByTestId("celebration-banner")).toBeNull();
  });
});
