import { useEffect } from "react";

export interface BuilderKeyboardBindings {
  onUndo: () => void;
  onRedo: () => void;
  onFitView: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onSelectAll: () => void;
  onDuplicateSelected: () => void;
  onShowShortcuts?: () => void;
}

const isMacPlatform = (): boolean => {
  if (typeof navigator === "undefined") return false;
  // `navigator.platform` is the v1 signal. When browsers retire it, swap to
  // `navigator.userAgentData?.platform` with platform as the fallback. Out of
  // scope for v1; see gotcha #6 in the brief.
  return /Mac|iPhone|iPod|iPad/.test(navigator.platform);
};

const isTextEditable = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false;
  if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return true;
  return target.isContentEditable;
};

/**
 * Global builder shortcuts: Cmd/Ctrl modified keys plus a no-modifier
 * `?` to open the shortcuts cheatsheet. Bound to `window` keydown so
 * shortcuts work whether or not the canvas itself has focus. Skips
 * editable elements (`<input>`, `<textarea>`, `contenteditable`) so
 * typing inside the AI prompt or inspector never undoes the canvas.
 */
export function useBuilderKeyboard(bindings: BuilderKeyboardBindings, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    const isMac = isMacPlatform();
    const handler = (e: KeyboardEvent) => {
      if (isTextEditable(e.target)) return;

      if (e.key === "?" && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        bindings.onShowShortcuts?.();
        return;
      }

      const mod = isMac ? e.metaKey : e.ctrlKey;
      if (!mod) return;

      switch (e.key.toLowerCase()) {
        case "z":
          e.preventDefault();
          if (e.shiftKey) bindings.onRedo();
          else bindings.onUndo();
          break;
        case "y":
          if (!isMac) {
            e.preventDefault();
            bindings.onRedo();
          }
          break;
        case "0":
          e.preventDefault();
          bindings.onFitView();
          break;
        case "=":
        case "+":
          e.preventDefault();
          bindings.onZoomIn();
          break;
        case "-":
          e.preventDefault();
          bindings.onZoomOut();
          break;
        case "a":
          e.preventDefault();
          bindings.onSelectAll();
          break;
        case "d":
          e.preventDefault();
          bindings.onDuplicateSelected();
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [enabled, bindings]);
}
