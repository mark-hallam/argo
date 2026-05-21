import Link from "next/link";
import "./marketing-nav.css";

/**
 * Shared marketing footer paired with `<MarketingNav>`. Renders the
 * canonical link strip + copyright line previously inlined in each
 * marketing page. Static — safe as a server component.
 */
export function MarketingFooter() {
  return (
    <footer className="mk-foot">
      <div className="mk-foot-links">
        <Link href="/landing">SKYCOT</Link>
        <Link href="/compare">Compare</Link>
        <Link href="/customers">Customers</Link>
        <Link href="/pricing">Pricing</Link>
        <Link href="/docs">Docs</Link>
        <Link href="/changelog">Changelog</Link>
        <a href="https://github.com/mark-hallam/skycot-packs">GitHub</a>
        <Link href="/privacy">Privacy</Link>
        <Link href="/terms">Terms</Link>
        <Link href="/security">Security</Link>
        <a href="mailto:hello@skycot.com">Contact</a>
      </div>
      <div>© 2026 SKYCOT · Made in Melbourne · skycot.com</div>
    </footer>
  );
}
