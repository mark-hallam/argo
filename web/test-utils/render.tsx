import type { ReactElement, ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, type RenderOptions, type RenderResult } from "@testing-library/react";

/**
 * Shared render seam for component/page tests.
 *
 * Wraps the tree in a fresh, retry-disabled `QueryClientProvider` so any
 * component that calls `useQuery`/`useQueryClient` mounts without the
 * "No QueryClient set, use QueryClientProvider to set one" harness error.
 * NextAuth's `useSession` is mocked globally in `vitest.setup.ts`, so no
 * `SessionProvider` is needed here.
 *
 * Use this instead of `@testing-library/react`'s bare `render` whenever the
 * component-under-test (or anything in its subtree) consumes react-query.
 * Each call gets its own client, so cache state never leaks between tests.
 */
export function renderWithProviders(
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">,
): RenderResult {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  }
  return render(ui, { wrapper: Wrapper, ...options });
}
