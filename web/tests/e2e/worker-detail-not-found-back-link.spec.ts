import { expect, test } from "@playwright/test";

/**
 * The worker-not-found shell back-link points at `/workers`, not
 * `/team`. The sidebar nav entrypoint is "Workers" → `/workers`, so
 * a 404 surface should send the operator back to the same surface
 * they clicked from for mental-model consistency. `/workers` itself
 * redirects server-side to `/team` when populated (covered by
 * `fix-workers-redirect-to-team-url-flicker`), so the user lands on
 * the same page either way — only the link intent changes.
 *
 * Spec source: stream
 * `fix-spa-landing-trailing-slash-and-worker-not-found-back-link`.
 * Audit finding: onboarding-hire-an-ai-teammate F14 [P2].
 */

// A never-existent worker id — outside the sample-data fallback set
// in `lib/api/sample-workers.ts`, so the layout falls through to the
// WorkerNotFound component.
const NOT_FOUND_ID = "00000000-0000-0000-0000-000000000000";

test("worker-not-found shell links back to /workers, not /team", async ({
  page,
}) => {
  await page.goto(`/workers/${NOT_FOUND_ID}/overview`);

  const shell = page.getByTestId("worker-not-found");
  await expect(shell).toBeVisible();

  const backLink = shell.getByRole("link", { name: /back to workers/i });
  await expect(backLink).toBeVisible();
  await expect(backLink).toHaveAttribute("href", "/workers");
});
