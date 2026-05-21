import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useBuilderKeyboard, type BuilderKeyboardBindings } from "./use-builder-keyboard";

function makeBindings(overrides: Partial<BuilderKeyboardBindings> = {}): BuilderKeyboardBindings {
  return {
    onUndo: vi.fn(),
    onRedo: vi.fn(),
    onFitView: vi.fn(),
    onZoomIn: vi.fn(),
    onZoomOut: vi.fn(),
    onSelectAll: vi.fn(),
    onDuplicateSelected: vi.fn(),
    onShowShortcuts: vi.fn(),
    ...overrides,
  };
}

function press(key: string, opts: { metaKey?: boolean; ctrlKey?: boolean; shiftKey?: boolean; target?: HTMLElement } = {}) {
  const event = new KeyboardEvent("keydown", {
    key,
    metaKey: opts.metaKey ?? false,
    ctrlKey: opts.ctrlKey ?? false,
    shiftKey: opts.shiftKey ?? false,
    cancelable: true,
    bubbles: true,
  });
  if (opts.target) {
    opts.target.dispatchEvent(event);
  } else {
    window.dispatchEvent(event);
  }
  return event;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useBuilderKeyboard", () => {
  it("Cmd+Z calls onUndo and prevents the default browser action", () => {
    Object.defineProperty(navigator, "platform", { value: "MacIntel", configurable: true });
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, true));
    const ev = press("z", { metaKey: true });
    expect(bindings.onUndo).toHaveBeenCalledTimes(1);
    expect(ev.defaultPrevented).toBe(true);
  });

  it("Cmd+Shift+Z calls onRedo", () => {
    Object.defineProperty(navigator, "platform", { value: "MacIntel", configurable: true });
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, true));
    press("z", { metaKey: true, shiftKey: true });
    expect(bindings.onRedo).toHaveBeenCalledTimes(1);
    expect(bindings.onUndo).not.toHaveBeenCalled();
  });

  it("Ctrl+Y on non-mac calls onRedo (Windows convention)", () => {
    Object.defineProperty(navigator, "platform", { value: "Win32", configurable: true });
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, true));
    press("y", { ctrlKey: true });
    expect(bindings.onRedo).toHaveBeenCalledTimes(1);
  });

  it("does NOT fire when typing in an <input>", () => {
    Object.defineProperty(navigator, "platform", { value: "MacIntel", configurable: true });
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, true));
    const input = document.createElement("input");
    document.body.appendChild(input);
    press("z", { metaKey: true, target: input });
    expect(bindings.onUndo).not.toHaveBeenCalled();
    document.body.removeChild(input);
  });

  it("'?' with no modifier opens the shortcuts modal", () => {
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, true));
    press("?");
    expect(bindings.onShowShortcuts).toHaveBeenCalledTimes(1);
  });

  it("does nothing when disabled", () => {
    Object.defineProperty(navigator, "platform", { value: "MacIntel", configurable: true });
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, false));
    press("z", { metaKey: true });
    press("0", { metaKey: true });
    press("?");
    expect(bindings.onUndo).not.toHaveBeenCalled();
    expect(bindings.onFitView).not.toHaveBeenCalled();
    expect(bindings.onShowShortcuts).not.toHaveBeenCalled();
  });

  it("Cmd+0 / Cmd+= / Cmd+- / Cmd+A / Cmd+D fire the matching handlers", () => {
    Object.defineProperty(navigator, "platform", { value: "MacIntel", configurable: true });
    const bindings = makeBindings();
    renderHook(() => useBuilderKeyboard(bindings, true));
    press("0", { metaKey: true });
    press("=", { metaKey: true });
    press("-", { metaKey: true });
    press("a", { metaKey: true });
    press("d", { metaKey: true });
    expect(bindings.onFitView).toHaveBeenCalledTimes(1);
    expect(bindings.onZoomIn).toHaveBeenCalledTimes(1);
    expect(bindings.onZoomOut).toHaveBeenCalledTimes(1);
    expect(bindings.onSelectAll).toHaveBeenCalledTimes(1);
    expect(bindings.onDuplicateSelected).toHaveBeenCalledTimes(1);
  });
});
