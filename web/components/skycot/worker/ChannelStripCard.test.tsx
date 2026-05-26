import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChannelStripCard } from "@/components/skycot/worker/ChannelStripCard";
import { aggregateByChannel, type ChannelAggregate } from "@/lib/api/audit";
import type { AuditRow } from "@/lib/types/audit";

const WORKER_ID = "wkr_mariah";

function Wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

function renderWithClient(node: ReactNode) {
  return render(<Wrapper>{node}</Wrapper>);
}

function row(
  channel: AuditRow["channel"],
  agoMs: number,
  i: number,
): AuditRow {
  const now = Date.now();
  return {
    id: `act_${i}`,
    timestamp: new Date(now - agoMs).toISOString(),
    worker: { id: WORKER_ID, name: "Mariah" },
    actionType: "message_sent",
    verb: "messaged buyer",
    status: "succeeded",
    channel,
  };
}

describe("ChannelStripCard", () => {
  it("renders one row per channel with counts, sorted by count desc", () => {
    const aggregates: ChannelAggregate[] = [
      { channel: "slack", count: 4, lastSeen: new Date(Date.now() - 30 * 60_000).toISOString() },
      { channel: "web", count: 2, lastSeen: new Date(Date.now() - 4 * 3600_000).toISOString() },
      { channel: "imessage", count: 1, lastSeen: new Date(Date.now() - 2 * 86400_000).toISOString() },
    ];
    renderWithClient(
      <ChannelStripCard
        workerId={WORKER_ID}
        workerName="Mariah"
        aggregatesOverride={aggregates}
      />,
    );
    expect(screen.getByTestId("card-channel-strip")).toBeInTheDocument();
    expect(screen.getByTestId("channel-row-slack")).toBeInTheDocument();
    expect(screen.getByTestId("channel-row-web")).toBeInTheDocument();
    expect(screen.getByTestId("channel-row-imessage")).toBeInTheDocument();
    expect(screen.getByTestId("channel-count-slack")).toHaveTextContent(/4 actions/);
    expect(screen.getByTestId("channel-count-imessage")).toHaveTextContent(/1 action(?!s)/);
  });

  it("links each row to /audit?worker=<id>&channel=<channel>", () => {
    const aggregates: ChannelAggregate[] = [
      { channel: "slack", count: 4, lastSeen: new Date().toISOString() },
      { channel: "imessage", count: 1, lastSeen: new Date().toISOString() },
    ];
    renderWithClient(
      <ChannelStripCard
        workerId={WORKER_ID}
        aggregatesOverride={aggregates}
      />,
    );
    const slackLink = screen.getByTestId("channel-row-link-slack");
    expect(slackLink.getAttribute("href")).toBe(
      `/audit?worker=${WORKER_ID}&channel=slack`,
    );
    const imsgLink = screen.getByTestId("channel-row-link-imessage");
    expect(imsgLink.getAttribute("href")).toBe(
      `/audit?worker=${WORKER_ID}&channel=imessage`,
    );
  });

  it("renders the connect-CTA when only the web channel has activity", () => {
    const aggregates: ChannelAggregate[] = [
      { channel: "web", count: 6, lastSeen: new Date().toISOString() },
    ];
    renderWithClient(
      <ChannelStripCard
        workerId={WORKER_ID}
        workerName="Mariah"
        aggregatesOverride={aggregates}
      />,
    );
    const empty = screen.getByTestId("channel-strip-empty");
    expect(empty).toHaveTextContent(/Mariah hasn't picked up Slack or email yet/);
    const cta = within(empty).getByTestId("channel-strip-connect-cta");
    expect(cta.getAttribute("href")).toBe("/settings/connectors");
  });

  it("renders the connect-CTA when there is no activity at all", () => {
    renderWithClient(
      <ChannelStripCard
        workerId={WORKER_ID}
        aggregatesOverride={[]}
      />,
    );
    expect(screen.getByTestId("channel-strip-empty")).toBeInTheDocument();
    expect(
      screen.queryByTestId("channel-row-slack"),
    ).not.toBeInTheDocument();
  });
});

describe("aggregateByChannel", () => {
  it("groups rows by channel, sorts count desc", () => {
    const rows: AuditRow[] = [
      row("slack", 60_000, 1),
      row("slack", 5 * 60_000, 2),
      row("slack", 10 * 60_000, 3),
      row("imessage", 2 * 86400_000, 5),
    ];
    const agg = aggregateByChannel(rows);
    expect(agg.map((a) => a.channel)).toEqual(["slack", "imessage"]);
    expect(agg[0]).toMatchObject({ channel: "slack", count: 3 });
    expect(agg[1]).toMatchObject({ channel: "imessage", count: 1 });
  });

  it("folds rows missing a `channel` into the web bucket", () => {
    const rows: AuditRow[] = [
      { ...row("web", 60_000, 1), channel: undefined },
      { ...row("web", 5 * 60_000, 2), channel: undefined },
      row("slack", 30 * 60_000, 3),
    ];
    const agg = aggregateByChannel(rows);
    const web = agg.find((a) => a.channel === "web");
    expect(web?.count).toBe(2);
    expect(agg.find((a) => a.channel === "slack")?.count).toBe(1);
  });

  it("returns the most recent timestamp for each channel as lastSeen", () => {
    const now = Date.now();
    const newer = new Date(now - 60_000).toISOString();
    const older = new Date(now - 5 * 86400_000).toISOString();
    const rows: AuditRow[] = [
      { ...row("slack", 5 * 86400_000, 1), timestamp: older },
      { ...row("slack", 60_000, 2), timestamp: newer },
    ];
    const agg = aggregateByChannel(rows);
    expect(agg[0].lastSeen).toBe(newer);
  });
});
