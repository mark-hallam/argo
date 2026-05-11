"use client";

import { useState } from "react";
import { Avatar } from "@/components/ui/Avatar";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import { StatusPill } from "@/components/ui/StatusPill";
import { NavDrawer, NavItem } from "@/components/ui/NavDrawer";
import { StatusCell, TextCell, TimestampCell } from "@/components/ui/DataCell";
import { ApprovalCard, type ApprovalDecision } from "@/components/skycot/ApprovalCard";
import { ThemePicker } from "@/components/skycot/ThemePicker";

/**
 * Demo / smoke page. Renders every primitive and the
 * load-bearing ApprovalCard. Theme picker swaps light/dark + the
 * Acme tenant theme live. This is the surface to point a real
 * user at first.
 */
export default function HomePage() {
  const [lastDecision, setLastDecision] = useState<ApprovalDecision | null>(null);
  const [cardDecision, setCardDecision] = useState<ApprovalDecision | null>(null);

  return (
    <div className="min-h-screen">
      <NavDrawer brand={<span>SKYCOT</span>}>
        <NavItem href="#" active>
          Approvals
        </NavItem>
        <NavItem href="#">Team</NavItem>
        <NavItem href="#">Workers</NavItem>
        <NavItem href="#">Audit</NavItem>
        <NavItem href="#">Settings</NavItem>
      </NavDrawer>

      <main className="md:pl-60">
        <div className="mx-auto max-w-4xl px-4 py-10 space-y-10">
          <header className="space-y-3">
            <h1 className="text-2xl font-semibold tracking-tight">Design system preview</h1>
            <p className="text-[var(--muted-foreground)] text-sm max-w-2xl">
              All primitives plus the load-bearing ApprovalCard. Toggle light/dark and
              switch between the SKYCOT default and the Acme tenant theme to see the
              CSS-variable theming in action.
            </p>
            <ThemePicker />
          </header>

          <section className="space-y-4">
            <h2 className="text-base font-semibold">ApprovalCard — the load-bearing surface</h2>
            <ApprovalCard
              worker={{ name: "Mariah" }}
              verb="wants to send this listing"
              detail={
                "Bauer Vapor X5, used 2x · $90. Reply to buyer: \"Yes still available, $90 firm. Pickup in Auckland this weekend.\""
              }
              consequenceIfYes="goes to Facebook Marketplace, visible to buyer in 30s."
              consequenceIfNo="I'll wait for your direction."
              reason="I'm asking because you said 'always check counter-offers' on May 4th."
              channel="slack"
              decision={cardDecision}
              onDecide={(d) => {
                setCardDecision(d);
                setLastDecision(d);
              }}
              onUndo={() => {
                setCardDecision(null);
              }}
            />
            {lastDecision ? (
              <p className="text-xs text-[var(--muted-foreground)]">
                Last decision: <strong>{lastDecision}</strong>
              </p>
            ) : null}
          </section>

          <section className="space-y-4">
            <h2 className="text-base font-semibold">Primitives</h2>

            <Card>
              <CardHeader>
                <CardTitle>Buttons</CardTitle>
                <CardDescription>All five variants in three sizes.</CardDescription>
              </CardHeader>
              <CardBody className="flex flex-wrap items-center gap-2">
                <Button variant="primary">Primary</Button>
                <Button variant="secondary">Secondary</Button>
                <Button variant="ghost">Ghost</Button>
                <Button variant="destructive">Destructive</Button>
                <Button variant="link">Link</Button>
                <Button loading>Loading</Button>
                <Button size="sm">Small</Button>
                <Button size="lg">Large</Button>
              </CardBody>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Status pills</CardTitle>
                <CardDescription>Semantic tones with optional dot.</CardDescription>
              </CardHeader>
              <CardBody className="flex flex-wrap gap-2">
                <StatusPill tone="success">Approved</StatusPill>
                <StatusPill tone="warning">Waiting</StatusPill>
                <StatusPill tone="error">Refused</StatusPill>
                <StatusPill tone="info">Working</StatusPill>
                <StatusPill tone="muted">Idle</StatusPill>
                <StatusPill tone="info" dot={false}>
                  No dot
                </StatusPill>
              </CardBody>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Avatars</CardTitle>
                <CardDescription>Four sizes with status rings.</CardDescription>
              </CardHeader>
              <CardBody className="flex flex-wrap items-center gap-4">
                <Avatar alt="Mariah" size="xs" status="working" />
                <Avatar alt="Mariah" size="sm" status="waiting" />
                <Avatar alt="Mariah" size="md" status="idle" />
                <Avatar alt="Mariah Chen" size="lg" status="blocked" />
              </CardBody>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Data cells</CardTitle>
                <CardDescription>Components for tables / timelines.</CardDescription>
              </CardHeader>
              <CardBody>
                <div className="grid grid-cols-4 gap-x-6 gap-y-2 text-xs text-[var(--muted-foreground)] pb-2 border-b border-[var(--border)]">
                  <span>Worker</span>
                  <span>Action</span>
                  <span>Status</span>
                  <span className="text-right">When</span>
                </div>
                <div className="grid grid-cols-4 gap-x-6 gap-y-3 pt-3">
                  <TextCell>Mariah</TextCell>
                  <TextCell>Sent listing</TextCell>
                  <StatusCell tone="success" label="Approved" />
                  <TimestampCell iso={new Date(Date.now() - 5 * 60_000).toISOString()} relative />

                  <TextCell>Priya</TextCell>
                  <TextCell>Drafted reply</TextCell>
                  <StatusCell tone="warning" label="Waiting" />
                  <TimestampCell iso={new Date(Date.now() - 32 * 60_000).toISOString()} relative />

                  <TextCell>Jamal</TextCell>
                  <TextCell>Refused refund</TextCell>
                  <StatusCell tone="error" label="Refused" />
                  <TimestampCell iso={new Date(Date.now() - 2 * 3600_000).toISOString()} relative />
                </div>
              </CardBody>
            </Card>
          </section>
        </div>
      </main>
    </div>
  );
}
