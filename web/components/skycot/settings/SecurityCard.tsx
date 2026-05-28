"use client";

import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export type SecurityCardTone = "neutral" | "on" | "warn";

interface SecurityCardRow {
  label: string;
  value: ReactNode;
}

export interface SecurityCardProps {
  icon: LucideIcon;
  title: string;
  blurb: string;
  statusLabel: string;
  statusTone?: SecurityCardTone;
  rows?: SecurityCardRow[];
  cta?: {
    label: string;
    disabled?: boolean;
    tooltip?: string;
    onClick?: () => void;
  };
  children?: ReactNode;
  testId?: string;
  span?: "one" | "two";
}

export default function SecurityCard({
  icon: Icon,
  title,
  blurb,
  statusLabel,
  statusTone = "neutral",
  rows,
  cta,
  children,
  testId,
  span = "one",
}: SecurityCardProps) {
  const pillClass =
    statusTone === "on"
      ? "security-card-pill security-card-pill-on"
      : statusTone === "warn"
        ? "security-card-pill security-card-pill-warn"
        : "security-card-pill";
  return (
    <section
      className={span === "two" ? "security-card security-card-wide" : "security-card"}
      data-testid={testId}
    >
      <div className="security-card-head">
        <div>
          <h2 className="security-card-title">
            <Icon size={18} aria-hidden /> {title}
          </h2>
          <p className="security-card-blurb">{blurb}</p>
        </div>
        <span className={pillClass} data-testid={testId ? `${testId}-pill` : undefined}>
          {statusLabel}
        </span>
      </div>

      {rows && rows.length > 0 ? (
        <dl className="security-card-rows" data-testid={testId ? `${testId}-rows` : undefined}>
          {rows.map((row) => (
            <div className="security-card-row" key={row.label}>
              <dt>{row.label}</dt>
              <dd>{row.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}

      {children}

      {cta ? (
        <div className="security-card-actions">
          <button
            type="button"
            className="security-card-btn"
            onClick={cta.onClick}
            disabled={cta.disabled}
            title={cta.tooltip}
            aria-label={cta.tooltip ? `${cta.label}. ${cta.tooltip}` : cta.label}
            data-testid={testId ? `${testId}-cta` : undefined}
          >
            {cta.label}
          </button>
          {cta.disabled && cta.tooltip ? (
            <span
              className="security-card-tooltip"
              data-testid={testId ? `${testId}-tooltip` : undefined}
            >
              {cta.tooltip}
            </span>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
