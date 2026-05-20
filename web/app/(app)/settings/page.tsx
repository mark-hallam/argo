"use client";

import Link from "next/link";
import { Building2, Bell, Lock, Key, Download, Plug, Receipt, Users } from "lucide-react";

/**
 * Settings landing — locked D top-tabs design (2026-05-19 memory).
 *
 * Brand-restrained register: horizontal top-tabs across the workspace's
 * settings surfaces. Each tab is a route. This V1 lists the available
 * settings groups as cards; richer sub-tab content lives in each child
 * route (currently only /settings/data-export ships).
 */

const TILES = [
  {
    href: "/settings/workspace",
    icon: <Building2 size={20} />,
    title: "Workspace",
    blurb: "Name, default vertical, time zone, mission billing email.",
    coming: true,
  },
  {
    href: "/settings/members",
    icon: <Users size={20} />,
    title: "Team & roles",
    blurb: "Invite teammates, manage roles (owner / admin / user / guest).",
    coming: false,
  },
  {
    href: "/settings/notifications",
    icon: <Bell size={20} />,
    title: "Notifications",
    blurb: "Approval routing, escalation channels, daily / weekly digests.",
    coming: true,
  },
  {
    href: "/settings/security",
    icon: <Lock size={20} />,
    title: "Security",
    blurb: "Sessions, audit log retention, encryption-at-rest, SSO (v1.x).",
    coming: true,
  },
  {
    href: "/settings/providers",
    icon: <Key size={20} />,
    title: "Provider keys",
    blurb: "Anthropic, OpenAI, custom providers. Per-worker overrides.",
    coming: true,
  },
  {
    href: "/settings/connectors",
    icon: <Plug size={20} />,
    title: "Connectors",
    blurb: "Slack, GitHub, Linear, Notion, Email. OAuth scopes per channel.",
    coming: true,
  },
  {
    href: "/billing",
    icon: <Receipt size={20} />,
    title: "Billing",
    blurb: "Subscription tier, payment method, invoices, usage limits.",
    coming: false,
  },
  {
    href: "/settings/data-export",
    icon: <Download size={20} />,
    title: "Data export",
    blurb: "Bundle export, audit-log download, GDPR data subject requests.",
    coming: false,
  },
];

export default function SettingsLandingPage() {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .settings-stage {
          padding: 48px 32px 96px;
          max-width: 1100px;
          margin: 0 auto;
        }
        .header {
          margin-bottom: 36px;
        }
        .eyebrow {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 8px;
        }
        .title {
          font-family: "Inter", sans-serif;
          font-size: 32px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 6px;
          letter-spacing: -0.01em;
        }
        .sub {
          font-size: 15px;
          color: var(--muted-foreground, #3f3f46);
          margin: 0;
        }
        .settings-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 14px;
        }
        @media (max-width: 700px) {
          .settings-grid {
            grid-template-columns: 1fr;
          }
        }
        .settings-tile {
          display: flex;
          gap: 16px;
          padding: 20px;
          border-radius: 12px;
          border: 1px solid var(--border, #ece9f5);
          background: white;
          text-decoration: none;
          color: inherit;
          transition: all 0.15s;
          position: relative;
        }
        .settings-tile:hover {
          border-color: rgba(139, 92, 246, 0.4);
          box-shadow: 0 6px 12px -4px rgba(139, 92, 246, 0.1);
        }
        .settings-tile-coming {
          opacity: 0.72;
          cursor: not-allowed;
        }
        .settings-tile-coming:hover {
          border-color: var(--border, #ece9f5);
          box-shadow: none;
        }
        .settings-tile-icon {
          width: 36px;
          height: 36px;
          border-radius: 9px;
          background: var(--background, #fafaff);
          border: 1px solid var(--border, #ece9f5);
          color: #8b5cf6;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .settings-tile-body {
          flex: 1;
          min-width: 0;
        }
        .settings-tile-title {
          font-weight: 600;
          font-size: 15px;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 4px;
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .settings-tile-blurb {
          font-size: 13px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.45;
          margin: 0;
        }
        .settings-coming-soon {
          font-family: "JetBrains Mono", monospace;
          font-size: 9.5px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.08em;
          background: var(--background, #fafaff);
          border: 1px solid var(--border, #ece9f5);
          padding: 2px 7px;
          border-radius: 999px;
          font-weight: 600;
        }
      `}</style>
      <div className="settings-stage">
        <header className="header">
          <div className="eyebrow">⚙ Workspace settings</div>
          <h1 className="title">Settings</h1>
          <p className="sub">Workspace, team, notifications, security, providers, and billing.</p>
        </header>

        <div className="settings-grid">
          {TILES.map((tile) => (
            <Tile key={tile.href} {...tile} />
          ))}
        </div>
      </div>
    </>
  );
}

function Tile({
  href,
  icon,
  title,
  blurb,
  coming,
}: {
  href: string;
  icon: React.ReactNode;
  title: string;
  blurb: string;
  coming: boolean;
}) {
  const content = (
    <>
      <div className="settings-tile-icon">{icon}</div>
      <div className="settings-tile-body">
        <h3 className="settings-tile-title">
          {title}
          {coming ? <span className="settings-coming-soon">v1.x</span> : null}
        </h3>
        <p className="settings-tile-blurb">{blurb}</p>
      </div>
    </>
  );

  if (coming) {
    return (
      <div className="tile coming" aria-disabled>
        {content}
      </div>
    );
  }
  return (
    <Link href={href} className="tile">
      {content}
    </Link>
  );
}
