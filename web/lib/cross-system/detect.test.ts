import { describe, expect, it } from "vitest";

import { detect, segmentText } from "./detect";

describe("detect — Stripe", () => {
  it("links dashboard URLs to the matching subpath", () => {
    const text =
      "Refund issued. See https://dashboard.stripe.com/refunds/re_3O7abcXYZ for details.";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("stripe");
    expect(matches[0].deepLink.url).toBe(
      "https://dashboard.stripe.com/refunds/re_3O7abcXYZ",
    );
  });

  it("links bare Stripe IDs only when preceded by Stripe-context keyword", () => {
    const text =
      "Refund processed for re_3O7abcXYZ. Unrelated chat: re_test_message.";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].raw).toBe("re_3O7abcXYZ");
    expect(matches[0].deepLink.url).toBe(
      "https://dashboard.stripe.com/refunds/re_3O7abcXYZ",
    );
  });

  it("does not link `re_test` without Stripe context", () => {
    const text = "Yesterday I sent re_testmessage to the team.";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — Slack", () => {
  it("links a workspace archive URL", () => {
    const text =
      "Approval thread: https://acmeco.slack.com/archives/C0123ABCD/p1700000000000001";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("slack");
    expect(matches[0].deepLink.url).toContain("/archives/C0123ABCD");
  });

  it("does not link non-Slack URLs masquerading as Slack-ish", () => {
    const text = "Visit https://slack-bot.example.com/archives/none for info.";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — GitHub", () => {
  it("links issue + pull-request URLs", () => {
    const text =
      "See https://github.com/acmeco/ops/issues/2912 and https://github.com/acmeco/ops/pull/3010.";
    const matches = detect(text);
    expect(matches).toHaveLength(2);
    expect(matches.map((m) => m.system)).toEqual(["github", "github"]);
  });

  it("does not link GitHub profile URLs (no issue or pull path)", () => {
    const text = "Profile: https://github.com/acmeco";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — Linear", () => {
  it("links Linear issue URLs", () => {
    const text =
      "Tracking: https://app.linear.app/acmeco/issue/SKY-184/payment-bug";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("linear");
    expect(matches[0].deepLink.url).toContain("SKY-184");
  });

  it("does not link bare Linear keys (gated to v1.5 manifest config)", () => {
    const text = "Ticket SKY-184 is in progress.";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — Salesforce", () => {
  it("links Lightning Opportunity URLs", () => {
    const text =
      "Deal: https://acmeco.lightning.force.com/lightning/r/Opportunity/0061S00000ABCDEFGH/view";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("salesforce");
  });

  it("does not link unknown Salesforce object types", () => {
    const text =
      "Setup: https://acmeco.lightning.force.com/lightning/setup/SetupOneHome/home";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — HubSpot", () => {
  it("links HubSpot contact-record URLs", () => {
    const text =
      "Customer: https://app.hubspot.com/contacts/12345678/record/0-1/55512345";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("hubspot");
  });

  it("does not link HubSpot marketing URLs", () => {
    const text =
      "Asset: https://app.hubspot.com/marketing/12345678/social";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — Zendesk", () => {
  it("links Zendesk ticket URLs", () => {
    const text =
      "Ticket: https://acmeco.zendesk.com/agent/tickets/29382 awaiting reply.";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("zendesk");
    expect(matches[0].deepLink.url).toContain("/tickets/29382");
  });

  it("does not link Zendesk help-center articles", () => {
    const text = "KB: https://acmeco.zendesk.com/hc/en-us/articles/12345";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — Jira", () => {
  it("links Atlassian Cloud browse URLs", () => {
    const text =
      "Tracking SKY-512: https://acmeco.atlassian.net/browse/SKY-512";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].system).toBe("jira");
    expect(matches[0].deepLink.url).toContain("SKY-512");
  });

  it("does not link Jira wiki / Confluence URLs", () => {
    const text =
      "Wiki: https://acmeco.atlassian.net/wiki/spaces/ENG/overview";
    expect(detect(text)).toEqual([]);
  });
});

describe("detect — overlap + ordering", () => {
  it("returns matches sorted by start offset", () => {
    const text =
      "First https://github.com/a/b/issues/1 and later https://app.linear.app/x/issue/SKY-2";
    const matches = detect(text);
    expect(matches.map((m) => m.system)).toEqual(["github", "linear"]);
    expect(matches[0].start).toBeLessThan(matches[1].start);
  });

  it("drops overlapping ranges, keeping the earlier match", () => {
    const text = "https://dashboard.stripe.com/refunds/re_overlap_3O7abc";
    const matches = detect(text);
    expect(matches).toHaveLength(1);
    expect(matches[0].deepLink.url).toBe(
      "https://dashboard.stripe.com/refunds/re_overlap_3O7abc",
    );
  });
});

describe("segmentText", () => {
  it("returns a single text segment when no matches are present", () => {
    expect(segmentText("hello world", [])).toEqual([
      { kind: "text", value: "hello world" },
    ]);
  });

  it("interleaves text and match segments in order", () => {
    const text = "see https://github.com/a/b/issues/1 thanks";
    const matches = detect(text);
    const segments = segmentText(text, matches);
    expect(segments).toHaveLength(3);
    expect(segments[0]).toEqual({ kind: "text", value: "see " });
    expect(segments[1].kind).toBe("match");
    expect(segments[2]).toEqual({ kind: "text", value: " thanks" });
  });

  it("returns an empty array for empty input", () => {
    expect(segmentText("", [])).toEqual([]);
  });
});

describe("detect — defensive", () => {
  it("returns empty for non-string input", () => {
    expect(detect(undefined as unknown as string)).toEqual([]);
    expect(detect(null as unknown as string)).toEqual([]);
  });

  it("truncates extremely long inputs to bound work", () => {
    const big = "x".repeat(8000) + " https://github.com/a/b/issues/1";
    expect(detect(big)).toEqual([]);
  });
});
