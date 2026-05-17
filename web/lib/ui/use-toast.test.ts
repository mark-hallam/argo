import { afterEach, describe, expect, it, vi } from "vitest";
import { toast as sonnerToast } from "sonner";
import { useToast } from "./use-toast";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(() => 1),
    error: vi.fn(() => 2),
    info: vi.fn(() => 3),
    warning: vi.fn(() => 4),
    loading: vi.fn(() => 5),
    promise: vi.fn(() => 6),
    dismiss: vi.fn(() => 0),
  },
}));

afterEach(() => {
  vi.clearAllMocks();
});

describe("useToast", () => {
  it("forwards success with the 4s default and merges options", () => {
    useToast().success("Worker hired", { description: "Maya is now active." });
    expect(sonnerToast.success).toHaveBeenCalledWith("Worker hired", {
      duration: 4000,
      description: "Maya is now active.",
    });
  });

  it("uses an 8s default for error toasts so failures stay readable", () => {
    useToast().error("Couldn't hire worker");
    expect(sonnerToast.error).toHaveBeenCalledWith("Couldn't hire worker", {
      duration: 8000,
    });
  });

  it("lets callers override the default duration", () => {
    useToast().warning("Quota near limit", { duration: 12_000 });
    expect(sonnerToast.warning).toHaveBeenCalledWith("Quota near limit", {
      duration: 12_000,
    });
  });

  it("loading does not impose an auto-dismiss duration", () => {
    useToast().loading("Hiring Maya...");
    expect(sonnerToast.loading).toHaveBeenCalledWith("Hiring Maya...", {});
  });

  it("promise forwards loading/success/error to Sonner", async () => {
    const p = Promise.resolve({ name: "Maya" });
    useToast().promise(p, {
      loading: "Hiring...",
      success: (w) => `${w.name} is ready.`,
      error: (e) => `Couldn't hire: ${String(e)}`,
    });
    const [arg0, data] = (sonnerToast.promise as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(arg0).toBe(p);
    expect(data.loading).toBe("Hiring...");
    expect(typeof data.success).toBe("function");
    expect(typeof data.error).toBe("function");
  });

  it("dismiss forwards to Sonner", () => {
    useToast().dismiss(42);
    expect(sonnerToast.dismiss).toHaveBeenCalledWith(42);
  });
});
