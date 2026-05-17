"use client";

import { toast as sonnerToast, type ExternalToast } from "sonner";

/**
 * Typed toast surface for the SPA. Wraps Sonner with a tighter API:
 * one entry per tone + a `promise` helper for async work. Sonner's
 * own functions are stable singletons, so this hook can be called
 * once per component without `useMemo`.
 *
 *   const { success, error, promise } = useToast();
 *   success("Worker hired", { description: "Maya is now active." });
 *   promise(api.hireWorker(input), {
 *     loading: "Hiring...",
 *     success: (w) => `${w.name} is ready.`,
 *     error: (e) => `Couldn't hire: ${e.message}`,
 *   });
 */

export interface ToastOptions extends Omit<ExternalToast, "description"> {
  description?: string;
}

export interface ToastPromiseMessages<T> {
  loading: string;
  success: string | ((data: T) => string);
  error: string | ((err: unknown) => string);
  description?: string | ((data: T) => string);
}

// Defaults tighten error/warning so failures stay visible long enough
// to read. Success/info inherit Sonner's 4s; loading stays until updated.
const D = { success: 4000, info: 4000, warning: 6000, error: 8000 };

export interface UseToastApi {
  success: (title: string, options?: ToastOptions) => string | number;
  error: (title: string, options?: ToastOptions) => string | number;
  info: (title: string, options?: ToastOptions) => string | number;
  warning: (title: string, options?: ToastOptions) => string | number;
  loading: (title: string, options?: ToastOptions) => string | number;
  promise: <T>(
    p: Promise<T>,
    messages: ToastPromiseMessages<T>,
    options?: ToastOptions,
  ) => string | number;
  dismiss: (id?: string | number) => void;
}

export function useToast(): UseToastApi {
  return {
    success: (title, opts) =>
      sonnerToast.success(title, { duration: D.success, ...opts }),
    error: (title, opts) =>
      sonnerToast.error(title, { duration: D.error, ...opts }),
    info: (title, opts) =>
      sonnerToast.info(title, { duration: D.info, ...opts }),
    warning: (title, opts) =>
      sonnerToast.warning(title, { duration: D.warning, ...opts }),
    loading: (title, opts) => sonnerToast.loading(title, opts ?? {}),
    promise: (p, m, opts) => {
      const result = sonnerToast.promise(p, {
        loading: m.loading,
        success: m.success,
        error: m.error,
        description: m.description,
        ...(opts ?? {}),
      });
      // Sonner's promise returns `(string | number) & { unwrap }` — a
      // tagged primitive at runtime. Narrow so callers can pass it to
      // `dismiss()`.
      return typeof result === "string" || typeof result === "number"
        ? result
        : 0;
    },
    dismiss: (id) => sonnerToast.dismiss(id),
  };
}
