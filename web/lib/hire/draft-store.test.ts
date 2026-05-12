import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { clearDraft, hasActiveDraft, loadDraft, saveDraft } from "./draft-store";
import {
  HIRE_DRAFT_STORAGE_KEY,
  HIRE_DRAFT_TTL_MS,
  HIRE_DRAFT_VERSION,
} from "./draft-types";

// Node 25 ships a built-in `localStorage` global that shadows happy-dom's
// when both are present; the built-in surfaces with no methods unless
// `--localstorage-file` is provided, breaking `window.localStorage.clear()`.
// Override per-test with an in-memory shim so tests are deterministic and
// independent of the host runtime's storage flags.
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

beforeEach(() => {
  installMemoryStorage();
  vi.restoreAllMocks();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("draft-store", () => {
  it("round-trips a saved draft via loadDraft", async () => {
    const ok = await saveDraft({
      step: 2,
      problem: "hire a marketplace receptionist",
      workerDraft: { name: "Mariah" },
    });
    expect(ok).toBe(true);
    const draft = await loadDraft();
    expect(draft).not.toBeNull();
    expect(draft?.step).toBe(2);
    expect(draft?.problem).toBe("hire a marketplace receptionist");
    expect(draft?.workerDraft).toEqual({ name: "Mariah" });
    expect(draft?.version).toBe(HIRE_DRAFT_VERSION);
    expect(typeof draft?.ts).toBe("number");
  });

  it("returns null when the saved draft is expired", async () => {
    await saveDraft({ step: 3 });
    const raw = window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw as string);
    parsed.ts = Date.now() - HIRE_DRAFT_TTL_MS - 60_000;
    window.localStorage.setItem(HIRE_DRAFT_STORAGE_KEY, JSON.stringify(parsed));
    const draft = await loadDraft();
    expect(draft).toBeNull();
    // expiry also clears the stale entry from storage
    expect(window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY)).toBeNull();
  });

  it("clearDraft removes the persisted entry", async () => {
    await saveDraft({ step: 4 });
    expect(window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY)).not.toBeNull();
    await clearDraft();
    expect(window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY)).toBeNull();
    expect(await loadDraft()).toBeNull();
  });

  it("preserves partial state across successive saves", async () => {
    await saveDraft({ step: 1, problem: "list a fridge on marketplace" });
    await saveDraft({ step: 2, problem: "list a fridge on marketplace", answers: { tone: "casual" } });
    const draft = await loadDraft();
    expect(draft?.step).toBe(2);
    expect(draft?.problem).toBe("list a fridge on marketplace");
    expect(draft?.answers).toEqual({ tone: "casual" });
  });

  it("returns null for malformed payloads", async () => {
    window.localStorage.setItem(HIRE_DRAFT_STORAGE_KEY, "not json{{");
    expect(await loadDraft()).toBeNull();
    window.localStorage.setItem(HIRE_DRAFT_STORAGE_KEY, JSON.stringify({ step: 99, ts: Date.now() }));
    expect(await loadDraft()).toBeNull();
    window.localStorage.setItem(
      HIRE_DRAFT_STORAGE_KEY,
      JSON.stringify({ version: 99, step: 1, ts: Date.now() }),
    );
    expect(await loadDraft()).toBeNull();
  });

  it("treats the most recent save as the winner (multi-tab last-write-wins)", async () => {
    await saveDraft({ step: 1, problem: "tab A" });
    // simulate another tab writing concurrently
    await saveDraft({ step: 2, problem: "tab B" });
    const draft = await loadDraft();
    expect(draft?.step).toBe(2);
    expect(draft?.problem).toBe("tab B");
  });

  it("returns null when stored version mismatches the current version (migration stub)", async () => {
    window.localStorage.setItem(
      HIRE_DRAFT_STORAGE_KEY,
      JSON.stringify({ version: 999, step: 2, ts: Date.now() }),
    );
    expect(await loadDraft()).toBeNull();
  });

  it("degrades gracefully when localStorage.setItem throws (quota / private mode)", async () => {
    const setItem = vi.spyOn(window.localStorage, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });
    const ok = await saveDraft({ step: 1 });
    expect(ok).toBe(false);
    expect(setItem).toHaveBeenCalled();
  });

  it("hasActiveDraft mirrors loadDraft truthiness", async () => {
    expect(await hasActiveDraft()).toBe(false);
    await saveDraft({ step: 2 });
    expect(await hasActiveDraft()).toBe(true);
    await clearDraft();
    expect(await hasActiveDraft()).toBe(false);
  });
});
