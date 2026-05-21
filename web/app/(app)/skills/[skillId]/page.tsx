"use client";
import "./page.css";

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
