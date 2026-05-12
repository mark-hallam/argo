"use client";

import { useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import {
  SkillRow,
  SourceBadge,
  TrustBadge,
} from "@/components/skycot/worker/SkillRow";
import type { LibrarySkill } from "@/lib/api/worker-skills";

/**
 * Library section — skills available to the worker but not yet
 * activated. Each row exposes a single `[Try it]` action; trust
 * scores below 70 trigger a "less reliable recently — try anyway?"
 * confirmation before enabling.
 */

const LOW_TRUST_THRESHOLD = 70;

export interface SkillsLibraryCardProps {
  skills: LibrarySkill[];
  onTry: (skill: LibrarySkill) => void;
}

export function SkillsLibraryCard({ skills, onTry }: SkillsLibraryCardProps) {
  const [pendingLowTrust, setPendingLowTrust] = useState<LibrarySkill | null>(null);

  const handleTry = (skill: LibrarySkill) => {
    if (skill.trustScore < LOW_TRUST_THRESHOLD) {
      setPendingLowTrust(skill);
      return;
    }
    onTry(skill);
  };

  return (
    <>
      <Card data-testid="card-skills-library">
        <CardHeader>
          <CardTitle>Library</CardTitle>
        </CardHeader>
        <CardBody className="pt-1">
          {skills.length === 0 ? (
            <p
              data-testid="skills-library-empty"
              className="text-sm text-[var(--muted-foreground)]"
            >
              No skills available beyond the ones in use. Add new
              patterns from the Learned section below.
            </p>
          ) : (
            <ul role="list" className="divide-y divide-[var(--border)]/60">
              {skills.map((skill) => (
                <SkillRow
                  key={skill.id}
                  id={skill.id}
                  title={skill.name}
                  version={skill.version}
                  meta={
                    <>
                      <TrustBadge trustScore={skill.trustScore} />
                      <SourceBadge source={skill.source} />
                    </>
                  }
                  actions={
                    <Button
                      variant="secondary"
                      size="sm"
                      data-testid={`skill-try-${skill.id}`}
                      onClick={() => handleTry(skill)}
                    >
                      Try it
                    </Button>
                  }
                />
              ))}
            </ul>
          )}
        </CardBody>
      </Card>

      <AlertDialog
        open={pendingLowTrust !== null}
        onOpenChange={(open) => {
          if (!open) setPendingLowTrust(null);
        }}
        title="Try this skill?"
        description={
          pendingLowTrust
            ? `This skill has been less reliable recently (trust: ${pendingLowTrust.trustScore}%). Try it anyway?`
            : ""
        }
        confirmLabel="Yes, try it"
        cancelLabel="Cancel"
        confirmVariant="primary"
        onConfirm={() => {
          if (pendingLowTrust) onTry(pendingLowTrust);
          setPendingLowTrust(null);
        }}
        onCancel={() => setPendingLowTrust(null)}
      />
    </>
  );
}

export function SkillsLibraryCardSkeleton() {
  return (
    <Card data-testid="card-skills-library-skeleton">
      <CardHeader>
        <div className="h-4 w-20 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        {[0, 1].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="size-4 rounded bg-[var(--muted)]" />
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-8 w-16 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
