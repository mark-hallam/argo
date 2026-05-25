import { describe, expect, it } from "vitest";
import { redactAuditRow } from "@/lib/redaction/redact";
import { VIEWER_POLICY_OPERATOR } from "@/lib/redaction/types";

describe("audit redaction legacy row guards", () => {
  it("normalizes legacy audit rows with no verb", () => {
    const out = redactAuditRow({ kind: "hipaa_compliance_check" }, VIEWER_POLICY_OPERATOR);

    expect(out.verb).toBe("hipaa_compliance_check");
    expect(out.actionType).toBe("hipaa_compliance_check");
    expect(out.status).toBe("failed");
    expect(out.worker.name).toBe("Unknown worker");
  });

  it("returns an unknown placeholder for an absent row", () => {
    const out = redactAuditRow(undefined, VIEWER_POLICY_OPERATOR);

    expect(out.id).toBe("unknown");
    expect(out.verb).toBe("unknown");
    expect(out.actionType).toBe("unknown");
  });
});
