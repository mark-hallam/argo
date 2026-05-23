"use client";

import "./page.css";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  Copy,
  Loader2,
  RefreshCw,
  Search,
  XCircle,
} from "lucide-react";
import {
  NOTIFY_CHANNELS,
  fallbackStatusFor,
  type NotifyChannelEntry,
  type ProbeStatus,
} from "@/lib/notify-channels/registry";

/**
 * `/settings/notify-channels` — operator-facing config surface for the
 * 11 approval notification channels shipped in `crates/argo-channels/`.
 *
 * Mirrors the D3 brand-loud-marketplace pattern from `/settings/connectors`
 * (PR #2138): hero search + cards grid + static catalog fallback. The
 * page attempts to fetch live probe data from `/api/doctor/channels`;
 * on 404 / error it falls back to the 2026-05-23 audit verdict so
 * operators always see a meaningful status pill. A follow-up brief
 * replaces the fallback with mandatory live data once the gateway
 * route ships.
 *
 * v1 ships read-mostly — env-var copy-blocks the operator pastes into
 * `~/.argo/cred.toml` or `direnv`. OAuth "Connect" buttons land once
 * `mission-credentials-per-tenant-master-key` provides the storage
 * substrate.
 */

const TRY_CHIPS = ["slack", "email", "imessage", "sms", "discord"];

const DOCTOR_ENDPOINT = "/api/doctor/channels";

interface DoctorChannelEntry {
  status?: string;
  detail?: string;
  retry_after?: string;
}

type DoctorResponse = Record<string, DoctorChannelEntry | unknown>;

interface ChannelLiveState {
  source: "live" | "fallback";
  status: ProbeStatus;
  detail: string | undefined;
}

function normalizeStatus(raw: unknown): ProbeStatus {
  if (typeof raw !== "string") return "UNKNOWN";
  const up = raw.trim().toUpperCase();
  if (up === "OK" || up === "MISSING" || up === "WARN" || up === "ERROR" || up === "SKIP") {
    return up;
  }
  return "UNKNOWN";
}

function liveStateFor(
  channel: NotifyChannelEntry,
  doctor: DoctorResponse | null,
): ChannelLiveState {
  const cell = doctor?.[channel.id];
  if (cell && typeof cell === "object") {
    const entry = cell as DoctorChannelEntry;
    return {
      source: "live",
      status: normalizeStatus(entry.status),
      detail: typeof entry.detail === "string" ? entry.detail : undefined,
    };
  }
  return {
    source: "fallback",
    status: fallbackStatusFor(channel.verdict),
    detail: undefined,
  };
}

function matchesQuery(c: NotifyChannelEntry, q: string): boolean {
  if (q.length === 0) return true;
  const needle = q.toLowerCase();
  return (
    c.name.toLowerCase().includes(needle) ||
    c.description.toLowerCase().includes(needle) ||
    c.id.toLowerCase().includes(needle) ||
    c.envVars.some((v) => v.name.toLowerCase().includes(needle))
  );
}

export default function NotifyChannelsPage() {
  const [query, setQuery] = useState("");
  const [doctor, setDoctor] = useState<DoctorResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastError, setLastError] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const fetchDoctor = useCallback(async () => {
    setLoading(true);
    setLastError(null);
    try {
      const res = await fetch(DOCTOR_ENDPOINT, { headers: { accept: "application/json" } });
      if (res.status === 404) {
        setDoctor(null);
        setLastError("Live probe endpoint not available yet — showing 2026-05-23 audit fallback.");
        return;
      }
      if (!res.ok) {
        setDoctor(null);
        setLastError(`Probe endpoint returned HTTP ${res.status}.`);
        return;
      }
      const json = (await res.json()) as DoctorResponse;
      setDoctor(json);
    } catch (e) {
      setDoctor(null);
      setLastError(e instanceof Error ? e.message : "Unknown probe error.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchDoctor();
  }, [fetchDoctor]);

  const filtered = useMemo(() => {
    const q = query.trim();
    return NOTIFY_CHANNELS.filter((c) => matchesQuery(c, q));
  }, [query]);

  const proIds = useMemo(
    () => NOTIFY_CHANNELS.filter((c) => c.proTier).map((c) => c.name).join(" · "),
    [],
  );

  function copyEnvVar(key: string) {
    void navigator.clipboard?.writeText(`${key}=`).catch(() => undefined);
    setCopiedKey(key);
    window.setTimeout(() => setCopiedKey((cur) => (cur === key ? null : cur)), 1400);
  }

  return (
    <div className="notify-stage" data-testid="notify-channels-page">
      <span className="notify-eyebrow">
        <span className="notify-eyebrow-dot" />
        APPROVAL CHANNELS
      </span>

      <h1 className="notify-hero">
        Reach approvers <strong>where they already are</strong>.
      </h1>

      <p className="notify-sub">
        {NOTIFY_CHANNELS.length} approval channels ship with SKYCOT — email, chat, SMS,
        voice, push, and the web inbox. Configure with env vars today; OAuth and the
        per-tenant secret vault land in a follow-up release.
      </p>

      <div className="notify-banner" data-testid="notify-pro-banner">
        <span className="notify-banner-tier">PRO</span>
        <span>
          Pro tier promises <strong>{proIds}</strong> out of the box. Other channels are
          available if you set the env vars but are not in the tier card.
        </span>
      </div>

      <div className="notify-search-row">
        <div className="notify-search-icon">
          <Search size={16} aria-hidden />
        </div>
        <input
          type="text"
          className="notify-search-input"
          placeholder="Search by channel, vendor, or env-var name…"
          aria-label="Search notification channels"
          data-testid="notify-search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <span className="notify-search-count" data-testid="notify-count">
          {NOTIFY_CHANNELS.length} total · {filtered.length} shown
        </span>
        <button
          type="button"
          className="notify-probe-button"
          onClick={() => void fetchDoctor()}
          disabled={loading}
          data-testid="notify-run-probe-all"
        >
          {loading ? <Loader2 size={13} className="notify-spin" /> : <RefreshCw size={13} />}
          {loading ? "Probing…" : "Run probe"}
        </button>
      </div>

      <div className="notify-try-row">
        <span className="notify-try-label">TRY</span>
        {TRY_CHIPS.map((chip) => (
          <button
            key={chip}
            type="button"
            className="notify-try-chip"
            onClick={() => setQuery(chip)}
          >
            {chip}
          </button>
        ))}
      </div>

      {lastError ? (
        <div className="notify-banner notify-banner-warn" data-testid="notify-fallback-banner">
          <AlertTriangle size={14} />
          <span>{lastError}</span>
        </div>
      ) : null}

      <div className="notify-section-label">CATALOG</div>

      {filtered.length === 0 ? (
        <div className="notify-empty-filter" data-testid="notify-empty-filter">
          No channels match &ldquo;{query}&rdquo;.{" "}
          <button type="button" onClick={() => setQuery("")}>Clear filter</button>
        </div>
      ) : (
        <div className="notify-grid">
          {filtered.map((c) => (
            <ChannelCard
              key={c.id}
              channel={c}
              state={liveStateFor(c, doctor)}
              expanded={expandedId === c.id}
              onToggle={() => setExpandedId((cur) => (cur === c.id ? null : c.id))}
              copiedKey={copiedKey}
              onCopy={copyEnvVar}
              loading={loading}
              onProbe={() => void fetchDoctor()}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface ChannelCardProps {
  channel: NotifyChannelEntry;
  state: ChannelLiveState;
  expanded: boolean;
  onToggle: () => void;
  copiedKey: string | null;
  onCopy: (key: string) => void;
  loading: boolean;
  onProbe: () => void;
}

function ChannelCard({
  channel,
  state,
  expanded,
  onToggle,
  copiedKey,
  onCopy,
  loading,
  onProbe,
}: ChannelCardProps) {
  const Icon = channel.Icon;
  return (
    <article
      className={`notify-card notify-card-${state.status.toLowerCase()}`}
      data-testid={`notify-card-${channel.id}`}
      data-channel-id={channel.id}
      data-status={state.status}
      data-verdict={channel.verdict}
      data-source={state.source}
    >
      <div className="notify-card-head">
        <div className={`notify-card-icon ${channel.tintClass}`}>
          <Icon size={20} aria-hidden />
        </div>
        <StatusPill status={state.status} source={state.source} />
      </div>

      <h2 className="notify-card-name">
        {channel.name}
        {channel.proTier ? <span className="notify-card-tier">Pro</span> : null}
      </h2>
      <p className="notify-card-blurb">{channel.description}</p>

      {channel.note ? (
        <div className="notify-card-note" data-testid={`notify-card-note-${channel.id}`}>
          <AlertTriangle size={12} aria-hidden />
          <span>{channel.note}</span>
        </div>
      ) : null}

      <button
        type="button"
        className="notify-card-expand"
        onClick={onToggle}
        aria-expanded={expanded}
        data-testid={`notify-card-expand-${channel.id}`}
      >
        <ChevronDown
          size={13}
          className={expanded ? "notify-card-expand-chevron-open" : undefined}
          aria-hidden
        />
        {expanded ? "Hide env vars" : "Show env vars"}
      </button>

      {expanded ? (
        <ul className="notify-card-envs" data-testid={`notify-card-envs-${channel.id}`}>
          {channel.envVars.map((env) => (
            <li key={env.name} className="notify-card-env">
              <button
                type="button"
                className="notify-card-env-key"
                onClick={() => onCopy(env.name)}
                aria-label={`Copy ${env.name}`}
                data-testid={`notify-card-copy-${channel.id}-${env.name}`}
              >
                <code>{env.name}</code>
                <Copy size={11} aria-hidden />
                {copiedKey === env.name ? (
                  <span className="notify-card-env-copied">Copied!</span>
                ) : null}
              </button>
              <p className="notify-card-env-hint">
                {env.required ? null : <em>Optional · </em>}
                {env.hint}
              </p>
            </li>
          ))}
        </ul>
      ) : null}

      <div className="notify-card-actions">
        <button
          type="button"
          className="notify-card-probe"
          onClick={onProbe}
          disabled={loading}
          data-testid={`notify-card-probe-${channel.id}`}
        >
          {loading ? <Loader2 size={12} className="notify-spin" /> : <RefreshCw size={12} />}
          Run probe
        </button>
      </div>
    </article>
  );
}

function StatusPill({ status, source }: { status: ProbeStatus; source: "live" | "fallback" }) {
  const className = `notify-pill notify-pill-${status.toLowerCase()}`;
  return (
    <span
      className={className}
      title={source === "fallback" ? "Static fallback (live probe unavailable)" : "Live probe"}
    >
      {iconForStatus(status)} {status}
      {source === "fallback" ? <span className="notify-pill-static">·static</span> : null}
    </span>
  );
}

function iconForStatus(status: ProbeStatus) {
  if (status === "OK") return <CheckCircle2 size={12} aria-hidden />;
  if (status === "ERROR" || status === "WARN") return <AlertTriangle size={12} aria-hidden />;
  if (status === "MISSING" || status === "UNKNOWN") return <XCircle size={12} aria-hidden />;
  return null;
}
