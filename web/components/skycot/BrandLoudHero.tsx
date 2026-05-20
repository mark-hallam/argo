"use client";

import { type CSSProperties } from "react";

/**
 * Brand-loud hero banner shared by Team, Approvals, and Cohorts.
 *
 * Renders the locked SKYCOT brand-loud register: gradient banner
 * (purple → pink → red), Audiowide hero title, JetBrains Mono eyebrow
 * + KPI labels, weight-900 stat numbers, animated shine sweeping
 * across the gradient. The per-surface variations (eyebrow dot color,
 * eyebrow label, whether the dot pulses, optional KPI tiles) flow
 * through props; the gradient, shadow, radius, and typography are
 * fixed by the brand recipe.
 */

export interface BrandLoudHeroKpi {
  label: string;
  value: string;
}

export interface BrandLoudHeroProps {
  eyebrowDotColor: string;
  eyebrowLabel: string;
  pulseDot?: boolean;
  title: string;
  titleAccent?: string;
  sub: string;
  kpis?: BrandLoudHeroKpi[];
  subMaxWidth?: number;
  shineDurationS?: number;
}

const DEFAULT_SUB_MAX_WIDTH = 460;
const DEFAULT_SHINE_DURATION_S = 5;

export function BrandLoudHero({
  eyebrowDotColor,
  eyebrowLabel,
  pulseDot = false,
  title,
  titleAccent,
  sub,
  kpis,
  subMaxWidth = DEFAULT_SUB_MAX_WIDTH,
  shineDurationS = DEFAULT_SHINE_DURATION_S,
}: BrandLoudHeroProps) {
  const rootStyle = {
    "--skycot-bl-dot-color": eyebrowDotColor,
    "--skycot-bl-sub-max-width": `${subMaxWidth}px`,
    "--skycot-bl-shine-duration": `${shineDurationS}s`,
  } as CSSProperties;

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .skycot-brandloud-hero {
          position: relative;
          margin: 24px 16px 0;
          padding: 28px 32px;
          border-radius: 18px;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 50%, #ff6b6b 100%);
          color: white;
          overflow: hidden;
          box-shadow: 0 16px 40px -16px rgba(139, 92, 246, 0.45);
        }
        .skycot-brandloud-hero::before {
          content: "";
          position: absolute;
          inset: 0;
          background: linear-gradient(
            115deg,
            transparent 30%,
            rgba(255, 255, 255, 0.18) 50%,
            transparent 70%
          );
          transform: translateX(-100%);
          animation: skycot-brandloud-shine var(--skycot-bl-shine-duration, 5s) cubic-bezier(0.42, 0, 0.58, 1) infinite;
          pointer-events: none;
        }
        @keyframes skycot-brandloud-shine {
          0%, 80%, 100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .skycot-brandloud-grid {
          position: relative;
          display: flex;
          align-items: flex-end;
          justify-content: space-between;
          gap: 32px;
          flex-wrap: wrap;
        }
        .skycot-brandloud-left {
          flex: 1;
          min-width: 260px;
        }
        .skycot-brandloud-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          color: rgba(255, 255, 255, 0.85);
          padding: 4px 12px;
          border-radius: 999px;
          background: rgba(255, 255, 255, 0.16);
          backdrop-filter: blur(8px);
          margin-bottom: 14px;
        }
        .skycot-brandloud-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--skycot-bl-dot-color, #f0fdf4);
        }
        .skycot-brandloud-eyebrow-dot.is-pulse {
          box-shadow: 0 0 6px rgba(255, 255, 255, 0.9);
          animation: skycot-brandloud-pulse 1.6s cubic-bezier(0.42, 0, 0.58, 1) infinite;
        }
        @keyframes skycot-brandloud-pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.45; }
        }
        .skycot-brandloud-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 36px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0.005em;
          margin: 0;
          color: white;
        }
        .skycot-brandloud-title-accent {
          background: linear-gradient(135deg, #fff5d6 0%, #fde68a 100%);
          -webkit-background-clip: text;
          background-clip: text;
          -webkit-text-fill-color: transparent;
          color: transparent;
        }
        .skycot-brandloud-sub {
          margin-top: 8px;
          font-size: 14px;
          font-weight: 400;
          color: rgba(255, 255, 255, 0.85);
          max-width: var(--skycot-bl-sub-max-width, 460px);
        }
        .skycot-brandloud-kpis {
          display: flex;
          gap: 28px;
        }
        .skycot-brandloud-kpi {
          text-align: right;
        }
        .skycot-brandloud-kpi-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 600;
          color: rgba(255, 255, 255, 0.75);
          margin-bottom: 4px;
        }
        .skycot-brandloud-kpi-value {
          font-family: "Inter", sans-serif;
          font-size: 36px;
          font-weight: 900;
          letter-spacing: -0.02em;
          color: white;
          line-height: 1;
        }
        @media (max-width: 720px) {
          .skycot-brandloud-hero {
            margin: 16px 12px 0;
            padding: 22px 22px;
          }
          .skycot-brandloud-title {
            font-size: 28px;
          }
          .skycot-brandloud-kpis {
            gap: 18px;
            width: 100%;
            justify-content: flex-start;
            margin-top: 16px;
          }
          .skycot-brandloud-kpi {
            text-align: left;
          }
          .skycot-brandloud-kpi-value {
            font-size: 28px;
          }
        }
      `}</style>
      <div className="skycot-brandloud-hero" style={rootStyle}>
        <div className="skycot-brandloud-grid">
          <div className="skycot-brandloud-left">
            <div className="skycot-brandloud-eyebrow">
              <span
                className={
                  pulseDot
                    ? "skycot-brandloud-eyebrow-dot is-pulse"
                    : "skycot-brandloud-eyebrow-dot"
                }
              />
              {eyebrowLabel}
            </div>
            <h1 className="skycot-brandloud-title">{renderTitle(title, titleAccent)}</h1>
            <p className="skycot-brandloud-sub">{sub}</p>
          </div>
          {kpis && kpis.length > 0 ? (
            <div className="skycot-brandloud-kpis">
              {kpis.map((k) => (
                <div className="skycot-brandloud-kpi" key={k.label}>
                  <div className="skycot-brandloud-kpi-label">{k.label}</div>
                  <div className="skycot-brandloud-kpi-value">{k.value}</div>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </>
  );
}

function renderTitle(title: string, accent: string | undefined) {
  if (!accent) return title;
  const idx = title.indexOf(accent);
  if (idx === -1) return title;
  const before = title.slice(0, idx);
  const after = title.slice(idx + accent.length);
  return (
    <>
      {before}
      <span className="skycot-brandloud-title-accent">{accent}</span>
      {after}
    </>
  );
}
