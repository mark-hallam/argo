"use client";

import { useState } from "react";
import Link from "next/link";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button, buttonVariants } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import {
  SkillRow,
  TrustBadge,
} from "@/components/skycot/worker/SkillRow";
import type { ActiveSkill } from "@/lib/api/worker-skills";

/**
 * Active skills section — skills currently bound to the worker. Each
 * row shows the trust badge, 7-day usage count, and three actions:
 * Disable (confirm dialog), View source (placeholder toast), and
 * Audit usage (deep-links into /audit filtered to this skill + this
 * worker — E5-a consumes the `skill` query param).
 */

export interface SkillsActiveCardProps {
  workerId: string;
  workerName: string;
  skills: ActiveSkill[];
  /** Called after the user confirms the Disable dialog. */
  onDisable: (skillId: string) => void;
  /** Called when "View source" is clicked. v1 = toast stub. */
  onViewSource: (skillId: string) => void;
}

export function SkillsActiveCard({
  workerId,
  workerName,
  skills,
  onDisable,
  onViewSource,
}: SkillsActiveCardProps) {
  const [pendingDisable, setPendingDisable] = useState<ActiveSkill | null>(null);

  return (
    <>
      <Card data-testid="card-skills-active">
        <CardHeader>
          <CardTitle>Active</CardTitle>
        </CardHeader>
        <CardBody className="pt-1">
          {skills.length === 0 ? (
            <p
              data-testid="skills-active-empty"
              className="text-sm text-[var(--muted-foreground)]"
            >
              {workerName} doesn&apos;t have any skills assigned yet.
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
                      <span
                        data-testid={`skill-usage-${skill.id}`}
                        className="text-xs text-[var(--muted-foreground)]"
                      >
                        used {skill.usageCount7d}× last 7d
                      </span>
                    </>
                  }
                  actions={
                    <>
                      <Button
                        variant="ghost"
                        size="sm"
                        data-testid={`skill-disable-${skill.id}`}
                        onClick={() => setPendingDisable(skill)}
                      >
                        Disable
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        data-testid={`skill-view-${skill.id}`}
                        onClick={() => onViewSource(skill.id)}
                      >
                        View source
                      </Button>
                      <Link
                        href={`/audit?worker=${encodeURIComponent(workerId)}&skill=${encodeURIComponent(skill.id)}`}
                        data-testid={`skill-audit-${skill.id}`}
                        className={buttonVariants({ variant: "ghost", size: "sm" })}
                      >
                        Audit usage
                      </Link>
                    </>
                  }
                />
              ))}
            </ul>
          )}
        </CardBody>
      </Card>

      <AlertDialog
        open={pendingDisable !== null}
        onOpenChange={(open) => {
          if (!open) setPendingDisable(null);
        }}
        title={`Disable ${pendingDisable?.name ?? "skill"}?`}
        description={`${workerName} will stop using this skill until it's re-enabled. The skill stays in her Library so you can switch it back on later.`}
        confirmLabel="Disable"
        cancelLabel="Keep"
        confirmVariant="destructive"
        onConfirm={() => {
          if (pendingDisable) onDisable(pendingDisable.id);
          setPendingDisable(null);
        }}
        onCancel={() => setPendingDisable(null)}
      />
    </>
  );
}

export function SkillsActiveCardSkeleton() {
  return (
    <Card data-testid="card-skills-active-skeleton">
      <CardHeader>
        <div className="h-4 w-16 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        {[0, 1, 2].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="size-4 rounded bg-[var(--muted)]" />
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-4 w-16 rounded-full bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
