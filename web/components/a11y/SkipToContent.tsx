/**
 * Skip-to-content link — WCAG 2.4.1 (Bypass Blocks).
 *
 * Visually hidden until focused. First focusable element in the
 * document, so the first Tab press from any page reveals it. Pressing
 * Enter (or activating the link) navigates to `#main`, which is wired
 * to an element with `tabindex="-1"` in each route group's shell so
 * focus actually lands on the page's main content.
 *
 * Brand-on visible state: Audiowide gradient pill matching the locked
 * SKYCOT brand identity (purple → pink). Lives in `top: 1rem` on focus.
 *
 * Mounted from `web/app/layout.tsx` so it covers every page in the app
 * without per-route plumbing.
 */
export function SkipToContent() {
  return (
    <a href="#main" className="skycot-skip-to-content" data-testid="skip-to-content">
      Skip to main content
    </a>
  );
}
