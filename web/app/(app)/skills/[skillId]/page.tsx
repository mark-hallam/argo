"use client";

import Link from "next/link";
import { use } from "react";

/**
 * Stub detail route for the skill marketplace cards. The full skill-
 * detail page (Skill IR + provenance + worker assignment) is a
 * separate brief; until that lands, the catalog cards fall through
 * here so they never deadlink.
 */
export default function SkillDetailStubPage({
  params,
}: {
  params: Promise<{ skillId: string }>;
}) {
  const { skillId } = use(params);
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx>{`
        .stub-stage {
          padding: 96px 32px;
          max-width: 720px;
          margin: 0 auto;
          text-align: center;
        }
        .stub-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 18px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid var(--border, #ece9f5);
          background: var(--background, #fafaff);
        }
        .stub-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 40px;
          line-height: 1.05;
          font-weight: 400;
          margin: 0 0 14px;
          color: var(--foreground, #1e1b4b);
        }
        .stub-title strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .stub-sub {
          font-size: 16px;
          color: var(--muted-foreground, #3f3f46);
          margin: 0 0 32px;
        }
        .stub-skill-id {
          font-family: "JetBrains Mono", monospace;
          font-size: 13px;
          color: var(--muted-foreground, #52525b);
          padding: 10px 16px;
          border-radius: 8px;
          background: var(--background, #fafaff);
          border: 1px solid var(--border, #ece9f5);
          display: inline-block;
          margin-bottom: 28px;
        }
        .stub-back {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          color: #8b5cf6;
          text-decoration: none;
          font-weight: 500;
        }
        .stub-back:hover {
          text-decoration: underline;
        }
      `}</style>
      <div className="stub-stage">
        <div className="stub-eyebrow">COMING IN v1.x</div>
        <h1 className="stub-title">
          Skill detail <strong>coming soon</strong>.
        </h1>
        <p className="stub-sub">
          The full skill view — provenance, version history, and worker assignment — ships in a
          follow-up brief. Meanwhile, here’s the skill you clicked:
        </p>
        <div className="stub-skill-id" data-testid="stub-skill-id">
          {skillId}
        </div>
        <div>
          <Link href="/skills" className="stub-back">
            ← Back to the marketplace
          </Link>
        </div>
      </div>
    </>
  );
}
