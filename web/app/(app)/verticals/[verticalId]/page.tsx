"use client";
import "./page.css";

import Link from "next/link";
import { use } from "react";
import { ArrowLeft } from "lucide-react";
import { useVertical } from "@/lib/hooks/use-verticals";

/**
 * Vertical-template detail page — brand-loud marketplace register.
 * Renders the gateway's `/verticals/<id>` response: the manifest
 * summary plus the `marketplace` envelope (workflows + actions +
 * KPIs). Mirrors the `/skills/<name>` detail-page layout so the
 * marketplace feels like one product, not two.
 */

interface PageProps {
  params: Promise<{ verticalId: string }>;
}

function decodeVerticalId(raw: string): string {
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

export default function VerticalDetailPage({ params }: PageProps) {
  const { verticalId } = use(params);
  const id = decodeVerticalId(verticalId);
  const { data, isLoading, isError, error } = useVertical(id);
  const vertical = data?.vertical;
  const marketplace = vertical?.marketplace;

  return (
    <div className="vd-stage">
      <Link href="/verticals" className="vd-back" data-testid="vertical-back">
        <ArrowLeft size={14} />
        Back to vertical marketplace
      </Link>

      {isLoading && (
        <div className="vd-panel" data-testid="vertical-loading">
          Loading vertical-template…
        </div>
      )}

      {isError && !isLoading && (
        <div
          className="vd-panel vd-panel-error"
          data-testid="vertical-error"
          role="alert"
        >
          Couldn’t load that vertical
          {error instanceof Error ? `: ${error.message}` : "."}
        </div>
      )}

      {!isLoading && !isError && vertical && (
        <>
          <div className="vd-header">
            <div className="vd-eyebrow">VERTICAL TEMPLATE · v{vertical.version}</div>
            <h1 className="vd-title">{vertical.name}</h1>
            <p className="vd-sub">{vertical.description}</p>
            <div className="vd-tag-row">
              {(vertical.tags ?? []).map((tag) => (
                <span key={tag} className="vd-tag">
                  {tag}
                </span>
              ))}
            </div>
          </div>

          <div className="vd-grid">
            <section className="vd-panel">
              <div className="vd-panel-label">WORKER ROLE</div>
              <div className="vd-role-headline">
                {marketplace?.role_label ?? vertical.name}
              </div>
              {vertical.provider ? (
                <p className="vd-meta-line">
                  Default model: <code>{vertical.provider.default}</code>
                  {vertical.provider.fallback ? (
                    <>
                      {" · fallback "}
                      <code>{vertical.provider.fallback}</code>
                    </>
                  ) : null}
                </p>
              ) : null}
              {vertical.tools ? (
                <p className="vd-meta-line">
                  <strong>{vertical.tools.allowed.length}</strong> tools allowed ·{" "}
                  <strong>{vertical.tools.denied.length}</strong> denied
                </p>
              ) : null}
            </section>

            <section className="vd-panel">
              <div className="vd-panel-label">WHAT’S IN THE BOX</div>
              <ul className="vd-stat-grid">
                <li>
                  <span className="vd-stat-num">
                    {marketplace?.action_count ?? 0}
                  </span>
                  <span className="vd-stat-label">Declared actions</span>
                </li>
                <li>
                  <span className="vd-stat-num">
                    {marketplace?.workflow_count ?? 0}
                  </span>
                  <span className="vd-stat-label">Sample workflows</span>
                </li>
                <li>
                  <span className="vd-stat-num">
                    {(vertical.memory_entries ?? []).length}
                  </span>
                  <span className="vd-stat-label">Memory entries</span>
                </li>
                <li>
                  <span className="vd-stat-num">
                    {(vertical.eval_suites ?? []).length}
                  </span>
                  <span className="vd-stat-label">Eval suites</span>
                </li>
              </ul>
            </section>
          </div>

          {marketplace && marketplace.actions.length > 0 ? (
            <section className="vd-panel">
              <div className="vd-panel-label">ILLUSTRATIVE WORKER ACTIONS</div>
              <ul className="vd-action-list" data-testid="vertical-actions">
                {marketplace.actions.map((action) => (
                  <li key={action.name} className="vd-action">
                    <div className="vd-action-head">
                      <code className="vd-action-name">{action.name}</code>
                      <span
                        className={`vd-risk vd-risk-${action.risk}`}
                        data-testid="vertical-action-risk"
                      >
                        {action.risk}
                      </span>
                      {action.requires_approval ? (
                        <span className="vd-approval">approval-gated</span>
                      ) : null}
                    </div>
                    {action.description ? (
                      <p className="vd-action-desc">{action.description}</p>
                    ) : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {marketplace && marketplace.workflows.length > 0 ? (
            <section className="vd-panel">
              <div className="vd-panel-label">SAMPLE WORKFLOWS</div>
              <ul className="vd-workflow-list" data-testid="vertical-workflows">
                {marketplace.workflows.map((w) => (
                  <li key={w.file} className="vd-workflow">
                    <code>{w.file}</code>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {(vertical.memory_entries ?? []).length > 0 ? (
            <section className="vd-panel">
              <div className="vd-panel-label">MEMORY SCHEMA</div>
              <ul className="vd-memory-list" data-testid="vertical-memory">
                {(vertical.memory_entries ?? []).map((entry) => (
                  <li key={entry.key} className="vd-memory-entry">
                    <code className="vd-memory-key">{entry.key}</code>
                    <span className="vd-memory-type">{entry.type}</span>
                    {entry.purpose ? (
                      <span className="vd-memory-purpose">{entry.purpose}</span>
                    ) : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {marketplace && marketplace.kpis.length > 0 ? (
            <section className="vd-panel">
              <div className="vd-panel-label">RELEASE-GATE KPIS</div>
              <ul className="vd-kpi-list" data-testid="vertical-kpis">
                {marketplace.kpis.map((kpi) => (
                  <li key={kpi.name} className="vd-kpi">
                    <code className="vd-kpi-name">{kpi.name}</code>
                    <span className="vd-kpi-meta">
                      {kpi.comparator} {kpi.threshold} ({kpi.kind} · {kpi.gate})
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </>
      )}
    </div>
  );
}
