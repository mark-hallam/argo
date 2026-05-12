import { describe, expect, it } from "vitest";
import type { Worker } from "@/lib/types/worker";
import {
  EMPTY_TEAM_FILTERS,
  activeTeamFilterCount,
  applyTeamFilters,
  clearTeamDimension,
  isTeamFilterEmpty,
  parseTeamFilters,
  serializeTeamFilters,
} from "./team";

function workerFixture(over: Partial<Worker> & Pick<Worker, "id" | "name" | "status">): Worker {
  return {
    role: { tagline: "does the thing", vertical: "ops" },
    ...over,
  } as Worker;
}

const SAMPLE: Worker[] = [
  workerFixture({ id: "a", name: "Mariah", status: "waiting", role: { tagline: "re-prices listings", vertical: "marketplace" } }),
  workerFixture({ id: "b", name: "Priya", status: "waiting", role: { tagline: "screens candidates", vertical: "recruiting" } }),
  workerFixture({ id: "c", name: "Jamal", status: "working", role: { tagline: "handles support tickets", vertical: "support" } }),
  workerFixture({ id: "d", name: "Nori", status: "idle", role: { tagline: "drafts product comms", vertical: "marketing" } }),
  workerFixture({ id: "e", name: "Sam", status: "blocked", role: { tagline: "watches the on-call queue", vertical: "ops" } }),
  workerFixture({ id: "f", name: "Rio", status: "paused", role: { tagline: "tidies sample replies", vertical: "support" } }),
];

describe("parseTeamFilters", () => {
  it("returns the empty default when no params are present", () => {
    expect(parseTeamFilters(new URLSearchParams(""))).toEqual(EMPTY_TEAM_FILTERS);
  });

  it("parses ?status= into a WorkerStatus", () => {
    expect(parseTeamFilters(new URLSearchParams("status=working"))).toEqual({
      status: "working",
      q: "",
    });
  });

  it("drops unknown ?status= values silently", () => {
    expect(parseTeamFilters(new URLSearchParams("status=not-a-status"))).toEqual(
      EMPTY_TEAM_FILTERS,
    );
  });

  it("parses ?q= and trims surrounding whitespace", () => {
    expect(parseTeamFilters(new URLSearchParams("q=%20mariah%20"))).toEqual({
      status: null,
      q: "mariah",
    });
  });

  it("parses both dimensions at once", () => {
    expect(parseTeamFilters(new URLSearchParams("status=working&q=jamal"))).toEqual({
      status: "working",
      q: "jamal",
    });
  });
});

describe("serializeTeamFilters", () => {
  it("emits an empty string for the empty filter", () => {
    expect(serializeTeamFilters(EMPTY_TEAM_FILTERS)).toBe("");
  });

  it("emits ?status= and ?q= in canonical order", () => {
    expect(
      serializeTeamFilters({ status: "working", q: "mariah" }),
    ).toBe("status=working&q=mariah");
  });

  it("round-trips through parse + serialize", () => {
    const filters = { status: "waiting" as const, q: "priya" };
    expect(parseTeamFilters(new URLSearchParams(serializeTeamFilters(filters)))).toEqual(filters);
  });
});

describe("isTeamFilterEmpty / activeTeamFilterCount", () => {
  it("reports the empty filter as empty", () => {
    expect(isTeamFilterEmpty(EMPTY_TEAM_FILTERS)).toBe(true);
    expect(activeTeamFilterCount(EMPTY_TEAM_FILTERS)).toBe(0);
  });

  it("counts each active dimension once", () => {
    expect(activeTeamFilterCount({ status: "working", q: "" })).toBe(1);
    expect(activeTeamFilterCount({ status: null, q: "mariah" })).toBe(1);
    expect(activeTeamFilterCount({ status: "working", q: "mariah" })).toBe(2);
  });
});

describe("applyTeamFilters", () => {
  it("returns the original list when filters are empty", () => {
    expect(applyTeamFilters(SAMPLE, EMPTY_TEAM_FILTERS)).toEqual(SAMPLE);
  });

  it("filters by status alone", () => {
    const result = applyTeamFilters(SAMPLE, { status: "waiting", q: "" });
    expect(result.map((w) => w.id).sort()).toEqual(["a", "b"]);
  });

  it("filters by case-insensitive name substring", () => {
    const result = applyTeamFilters(SAMPLE, { status: null, q: "MARI" });
    expect(result.map((w) => w.id)).toEqual(["a"]);
  });

  it("filters by case-insensitive tagline substring", () => {
    const result = applyTeamFilters(SAMPLE, { status: null, q: "on-call" });
    expect(result.map((w) => w.id)).toEqual(["e"]);
  });

  it("combines status AND search as intersection", () => {
    const result = applyTeamFilters(SAMPLE, { status: "paused", q: "tidies" });
    expect(result.map((w) => w.id)).toEqual(["f"]);
  });

  it("returns empty array when nothing matches the AND", () => {
    expect(applyTeamFilters(SAMPLE, { status: "blocked", q: "mariah" })).toEqual([]);
  });
});

describe("clearTeamDimension", () => {
  it("clears the status dimension only", () => {
    expect(
      clearTeamDimension({ status: "working", q: "mariah" }, "status"),
    ).toEqual({ status: null, q: "mariah" });
  });

  it("clears the search dimension only", () => {
    expect(
      clearTeamDimension({ status: "working", q: "mariah" }, "q"),
    ).toEqual({ status: "working", q: "" });
  });
});
