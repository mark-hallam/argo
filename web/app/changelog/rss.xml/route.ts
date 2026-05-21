import type { NextRequest } from "next/server";

/**
 * Minimal RSS feed for `/changelog`. Wave-4 audit flagged the RSS
 * link in the changelog header as a 404 — there was a `<link>` tag
 * but no underlying route. This file emits a static, valid RSS 2.0
 * feed seeded from the latest changelog entries on origin/main.
 *
 * The feed is intentionally hand-maintained for v1 (matches the
 * page-level WEEKS array). A CMS-backed feed lands in a follow-up
 * brief; until then, when the changelog page adds a new entry, add
 * a matching `<item>` here.
 */

interface RssItem {
  date: string;
  version: string;
  title: string;
  body: string;
}

const SITE = "https://skycot.com";

const ITEMS: RssItem[] = [
  {
    date: "2026-05-20",
    version: "v1.0.5",
    title: "Class-leading visual workflow builder shipped",
    body:
      "Drag-and-drop visual canvas with pan / zoom / mini-map. AI prompt bar for plain-English step proposals. Per-trigger config forms (webhook + schedule + email + form). Versioned + audit-logged on every save.",
  },
  {
    date: "2026-05-20",
    version: "v1.0.4",
    title: "Team & roles UX live",
    body:
      "Member management with four roles (Owner, Admin, Operator, Viewer), invite modal with copy-link affordance, and pending-invite tracking. Per-action role permissions are now visible end-to-end.",
  },
  {
    date: "2026-05-20",
    version: "v1.0.3",
    title: "Personality cards on worker profiles",
    body:
      "Every AI teammate now has a personality card on their profile — trait chips, signature behaviors, what they're great at, and when to escalate.",
  },
  {
    date: "2026-05-20",
    version: "v1.0.2",
    title: "Workflows catalog + visual viewer",
    body:
      "Public /workflows catalog with nine seeded workflows across sales, support, ops, and recruiting. Each opens into a read-only visual diagram so your team can browse what's possible before they hire.",
  },
  {
    date: "2026-05-20",
    version: "v1.0.1",
    title: "Brand-loud heroes across Approvals, Team, Cohorts",
    body:
      "Approvals inbox, Team & roles, and Cohorts pages all got the editorial-D3 brand-consistency pass — Audiowide hero typography, warm radial-blob backdrops, and gradient-emphasis on hero phrases.",
  },
  {
    date: "2026-05-19",
    version: "v1.0.0",
    title: "Editorial-A auth lock-in + globals.css brand-token swap",
    body:
      "Signup, verify-email, and signin migrated to the locked editorial-A design. globals.css now ships the canonical brand tokens.",
  },
];

function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function toRfc822(dateIso: string): string {
  return new Date(`${dateIso}T09:00:00Z`).toUTCString();
}

export function GET(_req: NextRequest): Response {
  const now = new Date().toUTCString();
  const itemsXml = ITEMS.map((item) => {
    const url = `${SITE}/changelog#${item.version}`;
    return [
      "    <item>",
      `      <title>${escapeXml(item.title)}</title>`,
      `      <link>${escapeXml(url)}</link>`,
      `      <guid isPermaLink="false">skycot-changelog-${escapeXml(item.version)}-${escapeXml(item.date)}</guid>`,
      `      <pubDate>${toRfc822(item.date)}</pubDate>`,
      `      <description>${escapeXml(item.body)}</description>`,
      "    </item>",
    ].join("\n");
  }).join("\n");

  const xml = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">',
    "  <channel>",
    "    <title>SKYCOT changelog</title>",
    `    <link>${SITE}/changelog</link>`,
    "    <description>What we shipped this week. Public changelog from SKYCOT.</description>",
    "    <language>en-au</language>",
    `    <lastBuildDate>${now}</lastBuildDate>`,
    `    <atom:link href="${SITE}/changelog/rss.xml" rel="self" type="application/rss+xml" />`,
    itemsXml,
    "  </channel>",
    "</rss>",
    "",
  ].join("\n");

  return new Response(xml, {
    status: 200,
    headers: {
      "content-type": "application/rss+xml; charset=utf-8",
      "cache-control": "public, max-age=300, stale-while-revalidate=900",
    },
  });
}
