"use client";

import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import {
  ArrowRight,
  CornerDownLeft,
  FileText,
  Layers,
  Mail,
  Search,
  Settings,
  Sparkles,
  UserPlus,
  Users,
  Workflow,
  Zap,
} from "lucide-react";

/**
 * SKYCOT cross-app command palette modal. Triggered by the global
 * `CommandPaletteProvider` which listens for ⌘K / Ctrl+K and renders
 * this component only when open (no DOM cost when closed).
 *
 * UX spec:
 * - Centred horizontally, 100px from top, ~640px wide on desktop, full-screen below 640px.
 * - Backdrop blur + dark overlay. Click outside or Escape closes.
 * - Search input with gradient icon + monospace placeholder.
 * - Results grouped: Workers / Workflows / Skills / Pages / Members.
 * - Empty search shows Recent (localStorage) + Quick actions.
 * - Arrow keys navigate, Enter activates, focus is trapped, restored on close.
 */

interface PaletteItem {
  id: string;
  category: ResultGroup;
  title: string;
  subtitle?: string;
  href?: string;
  action?: () => void;
  // Avatar initials (workers + members) or icon-only rendering otherwise.
  initials?: string;
  badge?: string;
  badgeColor?: string;
  // Optional secondary tag (e.g. vertical for workflow, category for skill)
  tag?: string;
  // Used for search matching beyond title/subtitle.
  keywords?: string[];
  // Icon component override.
  icon?: React.ReactNode;
}

type ResultGroup = "workers" | "workflows" | "skills" | "pages" | "members" | "quick" | "recent";

const GROUP_LABELS: Record<ResultGroup, string> = {
  workers: "Workers",
  workflows: "Workflows",
  skills: "Skills",
  pages: "Pages",
  members: "Members",
  quick: "Quick actions",
  recent: "Recent",
};

const GROUP_ORDER: ResultGroup[] = ["workers", "workflows", "skills", "pages", "members"];

// ---- seed data --------------------------------------------------------------
//
// Hardcoded to keep the palette feeling alive even when the gateway is
// offline. Workers are echoed from `lib/sample-team.ts` rather than imported
// to avoid coupling the palette to that data source (sample-team is in flux
// while the gateway endpoint lands).

const WORKERS: PaletteItem[] = [
  {
    id: "worker:mariah",
    category: "workers",
    title: "Mariah",
    subtitle: "lists and re-prices items for sale · marketplace",
    href: "/workers/wkr_01J0MARIAH",
    initials: "MA",
    badge: "waiting",
    badgeColor: "#f59e0b",
    keywords: ["marketplace", "listings", "reprice"],
  },
  {
    id: "worker:priya",
    category: "workers",
    title: "Priya",
    subtitle: "screens candidates and books interviews · recruiting",
    href: "/workers/wkr_01J0PRIYA",
    initials: "PR",
    badge: "waiting",
    badgeColor: "#f59e0b",
    keywords: ["recruiting", "candidate", "interview"],
  },
  {
    id: "worker:jamal",
    category: "workers",
    title: "Jamal",
    subtitle: "handles customer support tickets · support",
    href: "/workers/wkr_01J0JAMAL",
    initials: "JA",
    badge: "working",
    badgeColor: "#14b8a6",
    keywords: ["support", "tickets", "customer"],
  },
  {
    id: "worker:nori",
    category: "workers",
    title: "Nori",
    subtitle: "drafts product comms and announcements · marketing",
    href: "/workers/wkr_01J0NORI",
    initials: "NO",
    badge: "idle",
    badgeColor: "#71717a",
    keywords: ["marketing", "comms", "announcements"],
  },
  {
    id: "worker:rio",
    category: "workers",
    title: "Rio",
    subtitle: "books showings for the listing agent · real-estate",
    href: "/workers/wkr_01J0RIO",
    initials: "RI",
    badge: "paused",
    badgeColor: "#a855f7",
    keywords: ["real-estate", "showings", "listing"],
  },
  {
    id: "worker:sam-w",
    category: "workers",
    title: "Sam",
    subtitle: "watches the on-call queue · ops",
    href: "/workers/wkr_01J0SAM",
    initials: "SA",
    badge: "blocked",
    badgeColor: "#ef4444",
    keywords: ["ops", "on-call", "pagerduty"],
  },
];

const WORKFLOWS: PaletteItem[] = [
  {
    id: "wf:post_listing",
    category: "workflows",
    title: "post_listing",
    subtitle: "publish a new listing across channels",
    href: "/workflows/post_listing",
    tag: "marketplace",
    badge: "5 steps",
    keywords: ["listings", "publish", "marketplace"],
  },
  {
    id: "wf:qualify_lead",
    category: "workflows",
    title: "qualify_lead",
    subtitle: "score and route an inbound lead",
    href: "/workflows/qualify_lead",
    tag: "sales",
    badge: "4 steps",
    keywords: ["lead", "sales", "qualification"],
  },
  {
    id: "wf:screen_candidate",
    category: "workflows",
    title: "screen_candidate",
    subtitle: "shortlist and book the next interview",
    href: "/workflows/screen_candidate",
    tag: "recruiting",
    badge: "6 steps",
    keywords: ["recruiting", "candidate", "shortlist"],
  },
  {
    id: "wf:triage_ticket",
    category: "workflows",
    title: "triage_ticket",
    subtitle: "classify and assign a support ticket",
    href: "/workflows/triage_ticket",
    tag: "support",
    badge: "3 steps",
    keywords: ["support", "ticket", "triage"],
  },
  {
    id: "wf:approve_refund",
    category: "workflows",
    title: "approve_refund",
    subtitle: "route refund requests by amount and policy",
    href: "/workflows/approve_refund",
    tag: "support",
    badge: "5 steps",
    keywords: ["refund", "approval", "policy"],
  },
  {
    id: "wf:extract_invoice",
    category: "workflows",
    title: "extract_invoice",
    subtitle: "OCR and normalise an inbound invoice",
    href: "/workflows/extract_invoice",
    tag: "finance",
    badge: "4 steps",
    keywords: ["invoice", "finance", "ocr"],
  },
];

const SKILLS: PaletteItem[] = [
  {
    id: "skill:approval.route",
    category: "skills",
    title: "approval.route",
    subtitle: "route an action by amount + role",
    href: "/skills/approval.route",
    tag: "approvals",
    badge: "23 workers",
    keywords: ["approval", "routing"],
  },
  {
    id: "skill:invoice.extract",
    category: "skills",
    title: "invoice.extract",
    subtitle: "pull line items from PDFs and images",
    href: "/skills/invoice.extract",
    tag: "finance",
    badge: "11 workers",
    keywords: ["invoice", "extract", "finance"],
  },
  {
    id: "skill:candidate.screen",
    category: "skills",
    title: "candidate.screen",
    subtitle: "evaluate a candidate against the rubric",
    href: "/skills/candidate.screen",
    tag: "recruiting",
    badge: "9 workers",
    keywords: ["candidate", "screening", "recruiting"],
  },
  {
    id: "skill:deal.qualify",
    category: "skills",
    title: "deal.qualify",
    subtitle: "score an opportunity and choose a play",
    href: "/skills/deal.qualify",
    tag: "sales",
    badge: "14 workers",
    keywords: ["deal", "lead", "qualification"],
  },
  {
    id: "skill:support.triage",
    category: "skills",
    title: "support.triage",
    subtitle: "classify a ticket and pick an owner",
    href: "/skills/support.triage",
    tag: "support",
    badge: "18 workers",
    keywords: ["support", "ticket", "triage"],
  },
  {
    id: "skill:lead.enrich",
    category: "skills",
    title: "lead.enrich",
    subtitle: "add firmographics and intent signals",
    href: "/skills/lead.enrich",
    tag: "sales",
    badge: "12 workers",
    keywords: ["lead", "sales", "enrichment"],
  },
];

const PAGES: PaletteItem[] = [
  { id: "page:approvals", category: "pages", title: "Approvals", subtitle: "/", href: "/" },
  { id: "page:team", category: "pages", title: "Team", subtitle: "/team", href: "/team" },
  { id: "page:workers", category: "pages", title: "Workers", subtitle: "/workers", href: "/workers" },
  { id: "page:audit", category: "pages", title: "Audit", subtitle: "/audit", href: "/audit" },
  { id: "page:skills", category: "pages", title: "Skills", subtitle: "/skills", href: "/skills" },
  { id: "page:workflows", category: "pages", title: "Workflows", subtitle: "/workflows", href: "/workflows" },
  {
    id: "page:templates",
    category: "pages",
    title: "Templates",
    subtitle: "/workflows/templates",
    href: "/workflows/templates",
    keywords: ["workflow", "template"],
  },
  { id: "page:cohorts", category: "pages", title: "Cohorts", subtitle: "/cohorts", href: "/cohorts" },
  { id: "page:settings", category: "pages", title: "Settings", subtitle: "/settings", href: "/settings" },
  {
    id: "page:members",
    category: "pages",
    title: "Members",
    subtitle: "/settings/members",
    href: "/settings/members",
    keywords: ["team", "members", "invite"],
  },
  { id: "page:billing", category: "pages", title: "Billing", subtitle: "/billing", href: "/billing" },
  { id: "page:compare", category: "pages", title: "Compare", subtitle: "/compare", href: "/compare" },
  {
    id: "page:changelog",
    category: "pages",
    title: "Changelog",
    subtitle: "/changelog",
    href: "/changelog",
  },
  { id: "page:docs", category: "pages", title: "Docs", subtitle: "/docs", href: "/docs" },
  { id: "page:pricing", category: "pages", title: "Pricing", subtitle: "/pricing", href: "/pricing" },
  {
    id: "page:customers",
    category: "pages",
    title: "Customers",
    subtitle: "/customers",
    href: "/customers",
  },
];

const MEMBERS: PaletteItem[] = [
  {
    id: "member:sam-h",
    category: "members",
    title: "Sam Harlow",
    subtitle: "sam@acmeco.com · Owner",
    href: "/settings/members",
    initials: "SH",
    badge: "owner",
    badgeColor: "#8b5cf6",
    keywords: ["owner", "sam@acmeco.com"],
  },
  {
    id: "member:olivia",
    category: "members",
    title: "Olivia Chen",
    subtitle: "olivia@acmeco.com · Admin",
    href: "/settings/members",
    initials: "OC",
    badge: "admin",
    badgeColor: "#ec4899",
    keywords: ["admin", "olivia@acmeco.com"],
  },
  {
    id: "member:diego",
    category: "members",
    title: "Diego Alvarez",
    subtitle: "diego@acmeco.com · User",
    href: "/settings/members",
    initials: "DA",
    badge: "user",
    badgeColor: "#0891b2",
    keywords: ["user", "diego@acmeco.com"],
  },
  {
    id: "member:theo",
    category: "members",
    title: "Theo Park",
    subtitle: "theo@acmeco.com · User",
    href: "/settings/members",
    initials: "TP",
    badge: "user",
    badgeColor: "#0891b2",
    keywords: ["user", "theo@acmeco.com"],
  },
  {
    id: "member:naomi",
    category: "members",
    title: "Naomi Wright",
    subtitle: "naomi@auditor.gov · Guest",
    href: "/settings/members",
    initials: "NW",
    badge: "guest",
    badgeColor: "#71717a",
    keywords: ["guest", "auditor", "naomi@auditor.gov"],
  },
  {
    id: "member:aroha",
    category: "members",
    title: "Aroha Patel",
    subtitle: "aroha@acmeco.com · User",
    href: "/settings/members",
    initials: "AP",
    badge: "user",
    badgeColor: "#0891b2",
    keywords: ["user", "aroha@acmeco.com"],
  },
];

const ALL_ITEMS: PaletteItem[] = [
  ...WORKERS,
  ...WORKFLOWS,
  ...SKILLS,
  ...PAGES,
  ...MEMBERS,
];

const RECENT_STORAGE_KEY = "skycot:cmdk:recent";
const RECENT_LIMIT = 5;

interface RecentEntry {
  id: string;
  ts: number;
}

function loadRecent(): RecentEntry[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(RECENT_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (e): e is RecentEntry => typeof e?.id === "string" && typeof e?.ts === "number",
    );
  } catch {
    return [];
  }
}

function saveRecent(entries: RecentEntry[]): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(RECENT_STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // best-effort; storage quota or disabled — silently ignore
  }
}

function matches(item: PaletteItem, q: string): boolean {
  if (!q) return true;
  const needle = q.toLowerCase();
  if (item.title.toLowerCase().includes(needle)) return true;
  if (item.subtitle?.toLowerCase().includes(needle)) return true;
  if (item.tag?.toLowerCase().includes(needle)) return true;
  if (item.keywords?.some((k) => k.toLowerCase().includes(needle))) return true;
  return false;
}

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

export function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const router = useRouter();
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const previouslyFocused = useRef<HTMLElement | null>(null);
  const labelId = useId();
  const liveId = useId();

  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [recent, setRecent] = useState<RecentEntry[]>([]);

  // Build the quick-action list. These are inline because they need access to
  // `router` and `onClose`, which can't live in module scope.
  const quickActions: PaletteItem[] = useMemo(
    () => [
      {
        id: "quick:hire",
        category: "quick",
        title: "Hire a teammate",
        subtitle: "describe a role and SKYCOT compiles a worker",
        icon: <UserPlus size={15} />,
        action: () => router.push("/hire"),
      },
      {
        id: "quick:workflow",
        category: "quick",
        title: "Propose a workflow",
        subtitle: "open the workflow proposal canvas",
        icon: <Workflow size={15} />,
        action: () => router.push("/workflows"),
      },
      {
        id: "quick:invite",
        category: "quick",
        title: "Invite a member",
        subtitle: "send a magic-link invite to a human teammate",
        icon: <Mail size={15} />,
        action: () => router.push("/settings/members"),
      },
      {
        id: "quick:audit",
        category: "quick",
        title: "Run an audit query",
        subtitle: "filter the audit log by worker, action, or outcome",
        icon: <Sparkles size={15} />,
        action: () => router.push("/audit"),
      },
    ],
    [router],
  );

  // Sync recent on open so concurrent tabs are consistent.
  useEffect(() => {
    if (open) {
      setRecent(loadRecent());
      setQuery("");
      setActiveIndex(0);
      previouslyFocused.current = (document.activeElement as HTMLElement) ?? null;
    } else if (previouslyFocused.current) {
      try {
        previouslyFocused.current.focus();
      } catch {
        // element may be unmounted — ignore
      }
      previouslyFocused.current = null;
    }
  }, [open]);

  // Autofocus the input on mount of the modal.
  useEffect(() => {
    if (!open) return;
    // Defer a tick so the input is mounted and visible before focusing.
    const t = window.setTimeout(() => inputRef.current?.focus(), 10);
    return () => window.clearTimeout(t);
  }, [open]);

  // Compute the flat ordered list of items to render given the current query.
  const { groups, flat } = useMemo(() => {
    if (!query.trim()) {
      // Empty state: Recent + Quick actions only.
      const recentItems: PaletteItem[] = recent
        .map((entry) => ALL_ITEMS.find((it) => it.id === entry.id))
        .filter((it): it is PaletteItem => Boolean(it))
        .slice(0, RECENT_LIMIT)
        .map((it) => ({ ...it, category: "recent" as const }));

      const groupedEmpty: Array<{ key: ResultGroup; items: PaletteItem[] }> = [];
      if (recentItems.length > 0) {
        groupedEmpty.push({ key: "recent", items: recentItems });
      }
      groupedEmpty.push({ key: "quick", items: quickActions });
      const flatEmpty = groupedEmpty.flatMap((g) => g.items);
      return { groups: groupedEmpty, flat: flatEmpty };
    }

    const grouped: Array<{ key: ResultGroup; items: PaletteItem[] }> = [];
    for (const key of GROUP_ORDER) {
      const items = ALL_ITEMS.filter((it) => it.category === key && matches(it, query));
      if (items.length > 0) {
        grouped.push({ key, items });
      }
    }
    const flatted = grouped.flatMap((g) => g.items);
    return { groups: grouped, flat: flatted };
  }, [query, recent, quickActions]);

  // Clamp activeIndex into the new list when results change.
  useEffect(() => {
    if (activeIndex >= flat.length) {
      setActiveIndex(Math.max(0, flat.length - 1));
    }
  }, [flat.length, activeIndex]);

  const activate = useCallback(
    (item: PaletteItem) => {
      // Track in recent only for real items (skip quick actions, since they
      // already represent intent rather than a destination).
      if (item.category !== "quick") {
        const next: RecentEntry[] = [
          { id: item.id, ts: Date.now() },
          ...recent.filter((e) => e.id !== item.id),
        ].slice(0, RECENT_LIMIT);
        setRecent(next);
        saveRecent(next);
      }
      if (item.action) {
        item.action();
      } else if (item.href) {
        router.push(item.href);
      }
      onClose();
    },
    [onClose, recent, router],
  );

  // Scroll the active row into view as the user navigates with arrows.
  useEffect(() => {
    if (!open) return;
    const list = listRef.current;
    if (!list) return;
    const active = list.querySelector<HTMLElement>(`[data-cmdk-index="${activeIndex}"]`);
    if (active) {
      active.scrollIntoView({ block: "nearest" });
    }
  }, [activeIndex, open]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (flat.length === 0) return;
        setActiveIndex((i) => (i + 1) % flat.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        if (flat.length === 0) return;
        setActiveIndex((i) => (i - 1 + flat.length) % flat.length);
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const item = flat[activeIndex];
        if (item) activate(item);
        return;
      }
      // Cheap focus trap: Tab/Shift-Tab cycles within the modal. Because the
      // only focusable element we expose is the input, we just keep focus on
      // it — pressing Tab does nothing escapy.
      if (e.key === "Tab") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    },
    [activate, activeIndex, flat, onClose],
  );

  if (!open) return null;

  const resultCount = flat.length;
  const announce = query.trim()
    ? resultCount === 0
      ? "No matches"
      : `${resultCount} result${resultCount === 1 ? "" : "s"}`
    : "Recent and quick actions shown";

  return (
    <>
      <link
        href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .cmdk-overlay {
          position: fixed;
          inset: 0;
          background: rgba(30, 27, 75, 0.55);
          backdrop-filter: blur(6px);
          -webkit-backdrop-filter: blur(6px);
          display: flex;
          flex-direction: column;
          align-items: center;
          padding-top: 100px;
          z-index: 200;
        }
        .cmdk-modal {
          width: 100%;
          max-width: 640px;
          background: #ffffff;
          border-radius: 16px;
          box-shadow: 0 30px 80px -20px rgba(0, 0, 0, 0.45);
          border: 1px solid rgba(139, 92, 246, 0.15);
          overflow: hidden;
          font-family: "Inter", sans-serif;
          color: #1e1b4b;
          display: flex;
          flex-direction: column;
          max-height: calc(100vh - 140px);
        }
        @media (max-width: 640px) {
          .cmdk-overlay {
            padding-top: 0;
            align-items: stretch;
          }
          .cmdk-modal {
            border-radius: 0;
            max-width: 100%;
            max-height: 100vh;
            height: 100vh;
          }
        }
        .cmdk-search-row {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 16px 20px;
          border-bottom: 1px solid #ece9f5;
        }
        .cmdk-search-icon {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .cmdk-search-input {
          flex: 1;
          font-family: "JetBrains Mono", monospace;
          font-size: 14px;
          border: none;
          outline: none;
          background: transparent;
          color: #1e1b4b;
          letter-spacing: 0.01em;
        }
        .cmdk-search-input::placeholder {
          color: #a1a1aa;
        }
        .cmdk-kbd {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          font-weight: 600;
          color: #71717a;
          background: #f4f4f5;
          border: 1px solid #ece9f5;
          padding: 3px 7px;
          border-radius: 6px;
          letter-spacing: 0.04em;
        }
        .cmdk-list {
          overflow-y: auto;
          padding: 8px 6px 10px;
        }
        .cmdk-group {
          margin-top: 10px;
        }
        .cmdk-group:first-child {
          margin-top: 4px;
        }
        .cmdk-group-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #71717a;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 4px 14px 6px;
        }
        .cmdk-row {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 9px 14px;
          margin: 0 4px;
          border-radius: 9px;
          cursor: pointer;
          color: #1e1b4b;
          user-select: none;
        }
        .cmdk-row[data-active="true"] {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.1), rgba(236, 72, 153, 0.08));
          box-shadow: inset 0 0 0 1px rgba(139, 92, 246, 0.18);
        }
        .cmdk-row-icon {
          width: 30px;
          height: 30px;
          border-radius: 8px;
          display: grid;
          place-items: center;
          flex-shrink: 0;
          color: #6d28d9;
          background: rgba(139, 92, 246, 0.08);
        }
        .cmdk-row-icon.gradient {
          background: linear-gradient(135deg, #c4b5fd, #fbcfe8);
          color: #1e1b4b;
          font-family: "Inter", sans-serif;
          font-weight: 700;
          font-size: 11px;
          letter-spacing: 0.02em;
        }
        .cmdk-row-body {
          flex: 1;
          min-width: 0;
        }
        .cmdk-row-title {
          font-size: 14px;
          font-weight: 600;
          color: #1e1b4b;
          line-height: 1.25;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .cmdk-row-subtitle {
          font-size: 12px;
          color: #71717a;
          line-height: 1.4;
          margin-top: 2px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .cmdk-row-meta {
          display: flex;
          align-items: center;
          gap: 8px;
          flex-shrink: 0;
        }
        .cmdk-tag {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 700;
          color: #6d28d9;
          background: rgba(139, 92, 246, 0.1);
          padding: 3px 7px;
          border-radius: 999px;
        }
        .cmdk-badge {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.06em;
          text-transform: uppercase;
          font-weight: 700;
          color: white;
          padding: 3px 8px;
          border-radius: 999px;
        }
        .cmdk-enter {
          opacity: 0;
          color: #71717a;
          transition: opacity 120ms ease;
        }
        .cmdk-row[data-active="true"] .cmdk-enter {
          opacity: 1;
        }
        .cmdk-empty {
          padding: 36px 16px 32px;
          text-align: center;
          color: #71717a;
          font-size: 13.5px;
        }
        .cmdk-empty-title {
          font-family: "Inter", sans-serif;
          font-size: 16px;
          font-weight: 600;
          color: #1e1b4b;
          margin-bottom: 4px;
        }
        .cmdk-footer {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
          padding: 10px 16px;
          border-top: 1px solid #ece9f5;
          background: #fafaff;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #71717a;
          letter-spacing: 0.06em;
        }
        .cmdk-footer-hints {
          display: flex;
          gap: 14px;
          flex-wrap: wrap;
        }
        .cmdk-footer-hint {
          display: inline-flex;
          align-items: center;
          gap: 5px;
        }
        .cmdk-sr-only {
          position: absolute;
          width: 1px;
          height: 1px;
          padding: 0;
          margin: -1px;
          overflow: hidden;
          clip: rect(0, 0, 0, 0);
          white-space: nowrap;
          border: 0;
        }
      `}</style>
      <div
        className="cmdk-overlay"
        role="presentation"
        onMouseDown={(e) => {
          if (e.target === e.currentTarget) onClose();
        }}
      >
        <div
          className="cmdk-modal"
          role="dialog"
          aria-modal="true"
          aria-labelledby={labelId}
          onKeyDown={handleKeyDown}
        >
          <span id={labelId} className="cmdk-sr-only">
            Command palette — search workers, workflows, skills, settings
          </span>
          <span id={liveId} className="cmdk-sr-only" aria-live="polite" aria-atomic="true">
            {announce}
          </span>

          <div className="cmdk-search-row">
            <div className="cmdk-search-icon">
              <Search size={16} />
            </div>
            <input
              ref={inputRef}
              type="text"
              className="cmdk-search-input"
              placeholder="Search workers, workflows, skills, settings…"
              value={query}
              aria-controls={liveId}
              onChange={(e) => {
                setQuery(e.target.value);
                setActiveIndex(0);
              }}
            />
            <span className="cmdk-kbd">ESC</span>
          </div>

          <div className="cmdk-list" ref={listRef} role="listbox" aria-label="Command results">
            {flat.length === 0 ? (
              <div className="cmdk-empty">
                <div className="cmdk-empty-title">No matches</div>
                <div>Try a worker name, page, or skill ID.</div>
              </div>
            ) : (
              groups.map((group) => (
                <div className="cmdk-group" key={group.key}>
                  <div className="cmdk-group-label">{GROUP_LABELS[group.key]}</div>
                  {group.items.map((item) => {
                    const flatIndex = flat.indexOf(item);
                    const isActive = flatIndex === activeIndex;
                    return (
                      <div
                        key={item.id}
                        role="option"
                        aria-selected={isActive}
                        data-active={isActive ? "true" : "false"}
                        data-cmdk-index={flatIndex}
                        className="cmdk-row"
                        onMouseEnter={() => setActiveIndex(flatIndex)}
                        onMouseDown={(e) => {
                          // mousedown so we activate before the input blurs
                          e.preventDefault();
                          activate(item);
                        }}
                      >
                        <PaletteRowIcon item={item} />
                        <div className="cmdk-row-body">
                          <div className="cmdk-row-title">{item.title}</div>
                          {item.subtitle ? (
                            <div className="cmdk-row-subtitle">{item.subtitle}</div>
                          ) : null}
                        </div>
                        <div className="cmdk-row-meta">
                          {item.tag ? <span className="cmdk-tag">{item.tag}</span> : null}
                          {item.badge ? (
                            <span
                              className="cmdk-badge"
                              style={{
                                background: item.badgeColor ?? "#6d28d9",
                              }}
                            >
                              {item.badge}
                            </span>
                          ) : null}
                          <CornerDownLeft size={14} className="cmdk-enter" aria-hidden />
                        </div>
                      </div>
                    );
                  })}
                </div>
              ))
            )}
          </div>

          <div className="cmdk-footer">
            <div className="cmdk-footer-hints">
              <span className="cmdk-footer-hint">
                <ArrowRight size={11} /> ↑↓ navigate
              </span>
              <span className="cmdk-footer-hint">
                <CornerDownLeft size={11} /> enter to open
              </span>
              <span className="cmdk-footer-hint">esc to close</span>
            </div>
            <div>
              <span style={{ opacity: 0.7 }}>SKYCOT</span>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

interface RowIconProps {
  item: PaletteItem;
}

function PaletteRowIcon({ item }: RowIconProps) {
  if (item.initials) {
    return <div className="cmdk-row-icon gradient">{item.initials}</div>;
  }
  if (item.icon) {
    return <div className="cmdk-row-icon">{item.icon}</div>;
  }
  // Fall back to a category-appropriate icon.
  let glyph: React.ReactNode = <FileText size={15} />;
  switch (item.category) {
    case "workers":
      glyph = <Users size={15} />;
      break;
    case "workflows":
      glyph = <Workflow size={15} />;
      break;
    case "skills":
      glyph = <Sparkles size={15} />;
      break;
    case "members":
      glyph = <Users size={15} />;
      break;
    case "pages":
      glyph = item.id === "page:settings" ? <Settings size={15} /> : <FileText size={15} />;
      break;
    case "quick":
      glyph = <Zap size={15} />;
      break;
    case "recent":
      glyph = <Layers size={15} />;
      break;
  }
  return <div className="cmdk-row-icon">{glyph}</div>;
}
