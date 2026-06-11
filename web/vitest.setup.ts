import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";
import type { ReactNode } from "react";

/**
 * Root-fix for the next-auth harness debt.
 *
 * Many SPA components call `useSession()` (directly, or via
 * `useSkycotSession`) but most test files render those components without a
 * `<SessionProvider>`, so next-auth v5 throws
 * "`useSession` must be wrapped in a `<SessionProvider />`". That single
 * harness gap polluted the whole-suite signal and forced every web PR to
 * triage which red files were pre-existing.
 *
 * Provide an unauthenticated default session plus a passthrough
 * `SessionProvider` at the harness level so no test pays the wrapping cost.
 * `...actual` preserves the other exports (`signIn`, `signOut`, …) for any
 * test that imports them. A test that needs a specific session (a role, an
 * authenticated user) still declares its own
 * `vi.mock("next-auth/react", …)`, which overrides this module-level default
 * for that file.
 */
vi.mock("next-auth/react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("next-auth/react")>();
  return {
    ...actual,
    useSession: () => ({ data: null, status: "unauthenticated" }),
    SessionProvider: ({ children }: { children: ReactNode }) => children,
    getSession: async () => null,
  };
});
