"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { ArrowRight, MessageSquare } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";

/**
 * `/hire/done/<workerId>` viral-moment payoff. Wraps the new
 * teammate's name in a brand-loud Audiowide hero ("Meet ${name}.")
 * with a CSS-only confetti burst on mount, then a welcome card
 * carrying initials avatar + role + Slack / View-profile CTAs.
 *
 * Confetti and BrandLoudHero shine both honor
 * `prefers-reduced-motion`. Server-render emits a static frame; the
 * burst only fires after the post-mount `useEffect` resolves the
 * media query, which avoids a hydration mismatch on users who set
 * the OS preference.
 */

export interface HireDoneCelebrationProps {
  workerId: string;
  workerName: string;
  role?: string;
  /** Override the auto-derived initials (defaults to first two letters of `workerName`). */
  initials?: string;
  /** Mock Slack deep-link target. v1 uses a hash so we don't bake in workspace ids. */
  slackHref?: string;
}

const PARTICLE_COUNT = 36;

interface Particle {
  left: number;
  delay: number;
  duration: number;
  drift: number;
  color: string;
  rotate: number;
  size: number;
}

const PARTICLE_COLORS = ["#8b5cf6", "#a855f7", "#ec4899", "#f472b6", "#ff6b6b"];

function seededParticles(seed: string): Particle[] {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < seed.length; i += 1) {
    h = Math.imul(h ^ seed.charCodeAt(i), 16777619) >>> 0;
  }
  const next = () => {
    h = Math.imul(h ^ (h >>> 16), 2246822507) >>> 0;
    h = Math.imul(h ^ (h >>> 13), 3266489909) >>> 0;
    return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
  };
  return Array.from({ length: PARTICLE_COUNT }, () => ({
    left: Math.round(next() * 100),
    delay: Math.round(next() * 600),
    duration: Math.round(1400 + next() * 1400),
    drift: Math.round((next() - 0.5) * 220),
    color: PARTICLE_COLORS[Math.floor(next() * PARTICLE_COLORS.length)],
    rotate: Math.round((next() - 0.5) * 720),
    size: Math.round(6 + next() * 8),
  }));
}

function deriveInitials(name: string): string {
  const parts = name
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

export function HireDoneCelebration({
  workerId,
  workerName,
  role,
  initials,
  slackHref = "#slack-handoff",
}: HireDoneCelebrationProps) {
  const [mounted, setMounted] = useState(false);
  const [reduceMotion, setReduceMotion] = useState(false);

  useEffect(() => {
    setMounted(true);
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mql = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduceMotion(mql.matches);
    const handler = (e: MediaQueryListEvent) => setReduceMotion(e.matches);
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, []);

  const particles = useMemo(() => seededParticles(workerId || workerName), [workerId, workerName]);
  const showConfetti = mounted && !reduceMotion;
  const derivedInitials = initials ?? deriveInitials(workerName);
  const titleAccent = workerName;

  return (
    <div className="skycot-hire-done">
      <style jsx global>{`
        .skycot-hire-done {
          position: relative;
          min-height: 100vh;
          padding-bottom: 64px;
          background: radial-gradient(
              circle at 20% 0%,
              rgba(139, 92, 246, 0.18),
              transparent 55%
            ),
            radial-gradient(
              circle at 80% 10%,
              rgba(236, 72, 153, 0.16),
              transparent 60%
            ),
            #fdfcfb;
          overflow: hidden;
        }
        .skycot-hire-done-confetti {
          position: absolute;
          inset: 0;
          pointer-events: none;
          overflow: hidden;
        }
        .skycot-hire-done-confetti span {
          position: absolute;
          top: -24px;
          border-radius: 2px;
          opacity: 0;
          will-change: transform, opacity;
          animation: skycot-hire-done-fall var(--d, 2000ms) cubic-bezier(0.22, 0.61, 0.36, 1)
            var(--delay, 0ms) 1 forwards;
        }
        @keyframes skycot-hire-done-fall {
          0% {
            opacity: 0;
            transform: translate3d(0, -40px, 0) rotate(0deg) scale(0.6);
          }
          12% {
            opacity: 1;
          }
          100% {
            opacity: 0;
            transform: translate3d(var(--drift, 0px), 75vh, 0) rotate(var(--rot, 360deg))
              scale(1);
          }
        }
        @media (prefers-reduced-motion: reduce) {
          .skycot-hire-done-confetti {
            display: none;
          }
        }
        .skycot-hire-done-card {
          position: relative;
          margin: 28px 16px 0;
          padding: 32px;
          background: rgba(255, 255, 255, 0.92);
          backdrop-filter: blur(12px);
          border-radius: 20px;
          border: 1px solid rgba(30, 27, 75, 0.08);
          box-shadow: 0 24px 60px -32px rgba(139, 92, 246, 0.4);
          display: grid;
          gap: 28px;
          grid-template-columns: auto 1fr;
          align-items: center;
        }
        .skycot-hire-done-avatar {
          width: 96px;
          height: 96px;
          border-radius: 50%;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 60%, #ff6b6b 100%);
          color: white;
          font-family: var(--font-audiowide);
          font-size: 36px;
          font-weight: 400;
          letter-spacing: 0.02em;
          display: grid;
          place-items: center;
          box-shadow: 0 12px 28px -12px rgba(139, 92, 246, 0.6);
          flex-shrink: 0;
        }
        .skycot-hire-done-meta {
          min-width: 0;
        }
        .skycot-hire-done-name {
          font-family: "Inter", sans-serif;
          font-size: 22px;
          font-weight: 700;
          letter-spacing: -0.01em;
          color: #1e1b4b;
          margin: 0;
        }
        .skycot-hire-done-role {
          margin: 4px 0 0;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          color: rgba(30, 27, 75, 0.7);
        }
        .skycot-hire-done-ctas {
          display: flex;
          flex-wrap: wrap;
          gap: 12px;
          margin-top: 18px;
        }
        .skycot-hire-done-cta {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          border-radius: 10px;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 600;
          letter-spacing: 0.01em;
          text-decoration: none;
          transition: transform 0.15s ease, box-shadow 0.15s ease,
            background 0.15s ease;
        }
        .skycot-hire-done-cta.is-primary {
          color: white;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          box-shadow: 0 10px 24px -12px rgba(139, 92, 246, 0.6);
        }
        .skycot-hire-done-cta.is-primary:hover {
          transform: translateY(-1px);
          box-shadow: 0 14px 28px -12px rgba(139, 92, 246, 0.65);
        }
        .skycot-hire-done-cta.is-secondary {
          color: #1e1b4b;
          background: rgba(30, 27, 75, 0.05);
          border: 1px solid rgba(30, 27, 75, 0.1);
        }
        .skycot-hire-done-cta.is-secondary:hover {
          background: rgba(30, 27, 75, 0.08);
        }
        @media (max-width: 640px) {
          .skycot-hire-done-card {
            grid-template-columns: 1fr;
            justify-items: start;
            padding: 24px;
          }
          .skycot-hire-done-avatar {
            width: 72px;
            height: 72px;
            font-size: 28px;
          }
        }
      `}</style>

      {showConfetti ? (
        <div
          className="skycot-hire-done-confetti"
          aria-hidden
          data-testid="hire-done-confetti"
        >
          {particles.map((p, i) => (
            <span
              key={i}
              style={{
                left: `${p.left}%`,
                width: `${p.size}px`,
                height: `${p.size}px`,
                background: p.color,
                ["--delay" as never]: `${p.delay}ms`,
                ["--d" as never]: `${p.duration}ms`,
                ["--drift" as never]: `${p.drift}px`,
                ["--rot" as never]: `${p.rotate}deg`,
              }}
            />
          ))}
        </div>
      ) : null}

      <BrandLoudHero
        eyebrowDotColor="#fef3c7"
        eyebrowLabel="JUST HIRED · WELCOME"
        pulseDot
        title={`Meet ${workerName}.`}
        titleAccent={titleAccent}
        sub={`Your newest teammate is on the bench and ready to start. Say hi in Slack or jump into their profile to dial in their first week.`}
        subMaxWidth={520}
        shineDurationS={4}
      />

      <div className="skycot-hire-done-card" data-testid="hire-done-welcome-card">
        <div className="skycot-hire-done-avatar" aria-hidden>
          {derivedInitials}
        </div>
        <div className="skycot-hire-done-meta">
          <h2 className="skycot-hire-done-name">{workerName}</h2>
          {role ? <p className="skycot-hire-done-role">{role}</p> : null}
          <div className="skycot-hire-done-ctas">
            <a
              href={slackHref}
              className="skycot-hire-done-cta is-primary"
              data-testid="hire-done-slack-cta"
            >
              <MessageSquare size={16} aria-hidden />
              Say hi in Slack
            </a>
            <Link
              href={`/workers/${workerId}/overview`}
              className="skycot-hire-done-cta is-secondary"
              data-testid="hire-done-profile-cta"
            >
              View profile
              <ArrowRight size={16} aria-hidden />
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}
