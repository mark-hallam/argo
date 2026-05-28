/**
 * Demo-mode first-hire short-circuit pins.
 *
 * Closes brief `fix-demo-first-hire-discards-prompt-and-redirects-to-
 * sample-worker` (F1 + F2 + F7 from the 2026-05-27 onboarding e2e
 * audit). The prior `VERTICAL_TO_SAMPLE_WORKER` map silently routed
 * every demo hire to a fixed sample-roster persona (Mariah / Priya /
 * Jamal / Nori / Peta) and discarded the user's typed name +
 * description — the camera saw the wrong name on /workers/<id> and
 * the roster never grew. These pins confirm:
 *
 *  - the user's typed name is preserved on the returned worker
 *  - the synthesised hire is appended to the session-local demo
 *    roster (so `/team` + `/workers/<id>/overview` resolve it)
 *  - the returned id is a real UUIDv7, not a sample-roster id and
 *    not a `demo-worker-*` synthetic placeholder
 */

import { describe, expect, it, beforeEach, vi } from "vitest";
import {
  _clearDemoRosterForTests,
  listDemoWorkers,
  findDemoWorker,
} from "@/lib/api/demo-roster-store";
import { SAMPLE_PERSONA_IDS } from "@/lib/sample-personas";

vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => true,
  isDemoLive: () => false,
}));

import {
  DEMO_SUPPORTED_VERTICAL_IDS,
  firstHire,
} from "@/lib/onboarding/use-first-hire";

const UUID_V7_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

describe("firstHire (demo mode)", () => {
  beforeEach(() => {
    _clearDemoRosterForTests();
    vi.restoreAllMocks();
  });

  it("appends a Maya entry to the demo roster when description names her", async () => {
    const result = await firstHire({
      name: "Maya",
      description: "Maya, a marketing intern who drafts blog posts",
      verticalId: "marketing-intern",
    });

    expect(result.workerName).toBe("Maya");
    expect(result.pending).toBe(false);
    expect(result.workerId).toMatch(UUID_V7_PATTERN);

    const roster = listDemoWorkers();
    expect(roster).toHaveLength(1);
    expect(roster[0]?.name).toBe("Maya");
    expect(roster[0]?.id).toBe(result.workerId);
    expect(roster[0]?.role.vertical).toBe("marketing-intern");
  });

  it("preserves the typed name even when it shadows a sample persona", async () => {
    // Pre-fix behaviour routed `verticalId: 'recruiter'` to Priya
    // (SAMPLE_PRIYA_ID) regardless of the typed name. Now the typed
    // name wins and the id is fresh, so /workers/<new-id> resolves
    // to the user's hire — not the canonical sample-roster persona.
    const result = await firstHire({
      name: "Penny",
      description: "Penny screens applicants and books interviews",
      verticalId: "recruiter",
    });

    expect(result.workerName).toBe("Penny");
    expect(SAMPLE_PERSONA_IDS).not.toContain(result.workerId);
    const stored = findDemoWorker(result.workerId);
    expect(stored?.name).toBe("Penny");
  });

  it("stores the user's description as the worker tagline", async () => {
    const description = "Books property showings on weekdays after 4pm";
    const result = await firstHire({
      name: "Marcus",
      description,
      verticalId: "real-estate",
    });

    const stored = findDemoWorker(result.workerId);
    expect(stored?.role.tagline).toBe(description);
  });

  it("does not route unknown verticals to a sample-roster id", async () => {
    const result = await firstHire({
      name: "Ada",
      description: "Whatever",
      verticalId: "completely-bogus-vertical",
    });

    expect(SAMPLE_PERSONA_IDS).not.toContain(result.workerId);
    expect(result.workerName).toBe("Ada");
  });

  it("each call mints a fresh id (no idempotency collision between hires)", async () => {
    const first = await firstHire({
      name: "Maya",
      description: "Maya the marketer",
      verticalId: "marketing-intern",
    });
    const second = await firstHire({
      name: "Nora",
      description: "Nora the recruiter",
      verticalId: "recruiter",
    });

    expect(first.workerId).not.toBe(second.workerId);
    expect(listDemoWorkers()).toHaveLength(2);
  });

  it("DEMO_SUPPORTED_VERTICAL_IDS keeps ≥ 4 entries (F10 invariant) and every entry can be hired", async () => {
    // F10 (#2647) pinned the pick-vertical grid filter to the
    // verticals with a coherent persona match. Removing the
    // VERTICAL_TO_SAMPLE_WORKER map (this brief) must NOT shrink the
    // exported set — pick-vertical still filters its cards through it.
    expect(DEMO_SUPPORTED_VERTICAL_IDS.size).toBeGreaterThanOrEqual(4);
    for (const verticalId of DEMO_SUPPORTED_VERTICAL_IDS) {
      const result = await firstHire({
        name: "Anyone",
        description: `Hire for ${verticalId}`,
        verticalId,
      });
      expect(result.workerId).toMatch(UUID_V7_PATTERN);
      expect(SAMPLE_PERSONA_IDS).not.toContain(result.workerId);
    }
  });
});
