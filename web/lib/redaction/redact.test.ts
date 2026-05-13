import { describe, expect, it } from "vitest";
import {
  POLICY_MASK_ALL,
  POLICY_SHOW_ALL,
  maskedCategories,
  policyMasksAnything,
  redactAuditDetail,
  redactAuditRow,
  redactPII,
  resultHasMasks,
  textHasRedaction,
} from "./redact";
import { digitCount, luhnValid } from "./patterns";
import {
  APPROVER_SENTINEL,
  COUNTERPARTY_SENTINEL,
  categoriesLackedBy,
  redactCounterpartyHandles,
} from "./audit-rules";
import {
  ALL_PERMISSIONS,
  NO_PERMISSIONS,
  ROLE_DEFAULT_PERMISSIONS,
  VIEWER_POLICY_DELEGATOR,
  VIEWER_POLICY_OPERATOR,
  policyFromPermissions,
  type ViewerPolicy,
} from "./types";
import { resolveViewerPolicy } from "@/lib/hooks/use-viewer-policy";
import type { AuditRow } from "@/lib/types/audit";
import type { AuditDetail } from "@/lib/types/audit-detail";

describe("redactPII — email", () => {
  it("masks a single email with the [EMAIL] sentinel", () => {
    const { redacted, masked } = redactPII(
      "contact me at sam@acmeco.com",
      { ...POLICY_MASK_ALL },
    );
    expect(redacted).toBe("contact me at [EMAIL]");
    expect(masked.email).toBe(1);
  });

  it("returns the original text when the email policy is 'show'", () => {
    const { redacted, masked } = redactPII("ping kira+work@brand.co.nz", {
      ...POLICY_MASK_ALL,
      email: "show",
    });
    expect(redacted).toBe("ping kira+work@brand.co.nz");
    expect(masked.email).toBe(0);
  });

  it("masks multiple emails in one string", () => {
    const { redacted, masked } = redactPII(
      "a@x.co talked to b.c@y.io about prices",
      POLICY_MASK_ALL,
    );
    expect(redacted).toBe("[EMAIL] talked to [EMAIL] about prices");
    expect(masked.email).toBe(2);
  });
});

describe("redactPII — phone", () => {
  it("masks an NZ mobile-style number", () => {
    const { redacted, masked } = redactPII("call me on +64 21 555 0123", POLICY_MASK_ALL);
    expect(redacted).toBe("call me on [PHONE]");
    expect(masked.phone).toBe(1);
  });

  it("masks a US toll-free-style number with dashes", () => {
    const { redacted } = redactPII("dial 1-800-555-0199 for support", POLICY_MASK_ALL);
    expect(redacted).toBe("dial [PHONE] for support");
  });

  it("does NOT mask short digit runs (< 7 digits)", () => {
    const { redacted, masked } = redactPII("PIN 12345 in slot 3", POLICY_MASK_ALL);
    expect(redacted).toBe("PIN 12345 in slot 3");
    expect(masked.phone).toBe(0);
  });

  it("does NOT mask ISO dates (digit count too low)", () => {
    const { redacted, masked } = redactPII("delivered on 2026-05-11", POLICY_MASK_ALL);
    expect(masked.phone).toBe(0);
    expect(redacted).toContain("2026-05-11");
  });
});

describe("redactPII — card", () => {
  it("masks a Luhn-valid Visa test number", () => {
    // 4111 1111 1111 1111 is the canonical Visa test PAN; Luhn-valid.
    const { redacted, masked } = redactPII("paid with 4111 1111 1111 1111", POLICY_MASK_ALL);
    expect(redacted).toBe("paid with [CARD]");
    expect(masked.card).toBe(1);
  });

  it("rejects a 16-digit sequence that fails Luhn", () => {
    // Same shape, off-by-one to break the checksum.
    const { redacted, masked } = redactPII("order id 4111 1111 1111 1112", POLICY_MASK_ALL);
    // No mask from card; might still slip into phone? digitCount=16 > 15 so phone refuses.
    expect(masked.card).toBe(0);
    expect(redacted).toContain("4111 1111 1111 1112");
  });
});

describe("redactPII — IBAN + IRD", () => {
  it("masks an IBAN", () => {
    const { redacted, masked } = redactPII(
      "wire to DE89370400440532013000 by Friday",
      POLICY_MASK_ALL,
    );
    expect(redacted).toBe("wire to [IBAN] by Friday");
    expect(masked.iban).toBe(1);
  });

  it("masks an NZ IRD number (9-digit hyphenated)", () => {
    const { redacted, masked } = redactPII("IRD 123-456-789 on file", POLICY_MASK_ALL);
    expect(redacted).toBe("IRD [IRD] on file");
    expect(masked.ird).toBe(1);
  });

  it("masks an NZ IRD number (8-digit hyphenated)", () => {
    const { redacted } = redactPII("legacy IRD 12-345-678 still active", POLICY_MASK_ALL);
    expect(redacted).toContain("[IRD]");
  });
});

describe("redactPII — policy + helpers", () => {
  it("show-all policy returns the original text untouched", () => {
    const text = "email a@b.co, phone +64 21 555 0123, card 4111 1111 1111 1111";
    const { redacted, masked } = redactPII(text, POLICY_SHOW_ALL);
    expect(redacted).toBe(text);
    expect(resultHasMasks({ redacted, masked })).toBe(false);
  });

  it("masks only the categories whose policy says 'mask'", () => {
    const { redacted } = redactPII("a@b.co and +64 21 555 0123", {
      ...POLICY_SHOW_ALL,
      email: "mask",
    });
    expect(redacted).toBe("[EMAIL] and +64 21 555 0123");
  });

  it("policyMasksAnything / resultHasMasks / maskedCategories", () => {
    expect(policyMasksAnything(POLICY_MASK_ALL)).toBe(true);
    expect(policyMasksAnything(POLICY_SHOW_ALL)).toBe(false);

    const r = redactPII("a@b.co and IRD 123-456-789", POLICY_MASK_ALL);
    expect(resultHasMasks(r)).toBe(true);
    expect(maskedCategories(r).sort()).toEqual(["email", "ird"]);
  });
});

describe("patterns — Luhn + digitCount", () => {
  it("luhnValid accepts known-good Visa and Mastercard test pans", () => {
    expect(luhnValid("4111 1111 1111 1111")).toBe(true);
    expect(luhnValid("5500-0000-0000-0004")).toBe(true);
  });

  it("luhnValid rejects too-short / too-long sequences and non-digits", () => {
    expect(luhnValid("1234567890")).toBe(false); // too short
    expect(luhnValid("11111111111111111111")).toBe(false); // 20 digits
    expect(luhnValid("4111-abcd-1111-1111")).toBe(false);
  });

  it("digitCount ignores non-digit chars", () => {
    expect(digitCount("+64 21 555 0123")).toBe(11);
    expect(digitCount("no digits here")).toBe(0);
  });
});

// ============================================================================
// E5-g: audit-surface RBAC redaction
// ============================================================================

const sampleRow: AuditRow = {
  id: "act_001",
  timestamp: "2026-05-13T10:00:00Z",
  worker: { id: "wrk_1", name: "Echo" },
  actionType: "message_sent",
  verb: "Emailed sam@acmeco.com about renewal",
  status: "succeeded",
  approver: { id: "usr_42", name: "Petra Operator" },
  dataClassification: "confidential",
};

const sampleDetail: AuditDetail = {
  ...sampleRow,
  inputs: {
    triggerKind: "message",
    triggerSummary: "Buyer mailed sam@acmeco.com from @diego.alvarez",
    predecessor: { id: "act_000", verb: "Received reply from @sam.k" },
  },
  decisionChain: [
    { n: 1, title: "Read", body: "Customer phone +64 21 555 0123 surfaced" },
    { n: 2, title: "Plan", body: "Reply with quote — no PII" },
  ],
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04" },
    promptTemplate: { id: "pt_email_quote", hash: "sha256:abc" },
    skill: { id: "sk_email", version: "1.2" },
    memorySnapshot: { id: "mem_99" },
    policy: { id: "po_default", version: "3", hash: "sha256:def" },
  },
  output: {
    kind: "text",
    body: "Replied to sam@acmeco.com, copying @diego.alvarez",
    followUp: { id: "act_002", verb: "Awaiting reply from @sam.k" },
  },
};

describe("redactCounterpartyHandles", () => {
  it("masks @handles in prose", () => {
    const { redacted, count } = redactCounterpartyHandles("ping @sam.k for follow-up");
    expect(redacted).toBe(`ping ${COUNTERPARTY_SENTINEL} for follow-up`);
    expect(count).toBe(1);
  });

  it("masks vendor#-style handles", () => {
    const { redacted } = redactCounterpartyHandles("escalated to vendor#acme yesterday");
    expect(redacted).toBe(`escalated to ${COUNTERPARTY_SENTINEL} yesterday`);
  });

  it("leaves bare email addresses alone (handled by redactPII)", () => {
    const { redacted, count } = redactCounterpartyHandles("ping sam@acmeco.com");
    expect(redacted).toBe("ping sam@acmeco.com");
    expect(count).toBe(0);
  });
});

describe("redactAuditRow", () => {
  it("PII-redacts the verb when masking is in force", () => {
    const out = redactAuditRow(sampleRow, VIEWER_POLICY_DELEGATOR);
    expect(out.verb).toBe("Emailed [EMAIL] about renewal");
  });

  it("preserves the verb for an operator", () => {
    const out = redactAuditRow(sampleRow, VIEWER_POLICY_OPERATOR);
    expect(out.verb).toBe(sampleRow.verb);
  });

  it("masks approver name when unredactedApprover is false", () => {
    const out = redactAuditRow(sampleRow, VIEWER_POLICY_DELEGATOR);
    expect(out.approver?.name).toBe(APPROVER_SENTINEL);
    expect(out.approver?.id).toBe("usr_42");
  });

  it("keeps approver name when unredactedApprover is true", () => {
    const out = redactAuditRow(sampleRow, VIEWER_POLICY_OPERATOR);
    expect(out.approver?.name).toBe("Petra Operator");
  });

  it("hides dataClassification when unredactedClassification is false", () => {
    const out = redactAuditRow(sampleRow, VIEWER_POLICY_DELEGATOR);
    expect(out.dataClassification).toBeUndefined();
  });

  it("keeps dataClassification when unredactedClassification is true", () => {
    const out = redactAuditRow(sampleRow, VIEWER_POLICY_OPERATOR);
    expect(out.dataClassification).toBe("confidential");
  });

  it("masks counterparty handles inside the verb", () => {
    const row: AuditRow = { ...sampleRow, verb: "Pinged @sam.k about the deal" };
    const out = redactAuditRow(row, VIEWER_POLICY_DELEGATOR);
    expect(out.verb).toContain(COUNTERPARTY_SENTINEL);
  });

  it("is pure (does not mutate input)", () => {
    const before = JSON.stringify(sampleRow);
    redactAuditRow(sampleRow, VIEWER_POLICY_DELEGATOR);
    expect(JSON.stringify(sampleRow)).toBe(before);
  });
});

describe("redactAuditDetail", () => {
  it("redacts triggerSummary and predecessor.verb", () => {
    const out = redactAuditDetail(sampleDetail, VIEWER_POLICY_DELEGATOR);
    expect(out.inputs.triggerSummary).toContain("[EMAIL]");
    expect(out.inputs.triggerSummary).toContain(COUNTERPARTY_SENTINEL);
    expect(out.inputs.predecessor?.verb).toContain(COUNTERPARTY_SENTINEL);
  });

  it("redacts every decisionChain body", () => {
    const out = redactAuditDetail(sampleDetail, VIEWER_POLICY_DELEGATOR);
    expect(out.decisionChain[0].body).toBe("Customer phone [PHONE] surfaced");
    expect(out.decisionChain[1].body).toBe("Reply with quote — no PII");
  });

  it("redacts string output.body", () => {
    const out = redactAuditDetail(sampleDetail, VIEWER_POLICY_DELEGATOR);
    expect(out.output.body).toContain("[EMAIL]");
    expect(out.output.body).toContain(COUNTERPARTY_SENTINEL);
  });

  it("recursively redacts object output.body", () => {
    const detail: AuditDetail = {
      ...sampleDetail,
      output: {
        ...sampleDetail.output,
        kind: "json",
        body: {
          to: "sam@acmeco.com",
          meta: { handle: "@diego.alvarez", phone: "+64 21 555 0123" },
        },
      },
    };
    const out = redactAuditDetail(detail, VIEWER_POLICY_DELEGATOR);
    const body = out.output.body as Record<string, unknown>;
    expect(body.to).toBe("[EMAIL]");
    const meta = body.meta as Record<string, unknown>;
    expect(meta.handle).toBe(COUNTERPARTY_SENTINEL);
    expect(meta.phone).toBe("[PHONE]");
  });

  it("preserves provenance untouched (hashes are not PII)", () => {
    const out = redactAuditDetail(sampleDetail, VIEWER_POLICY_DELEGATOR);
    expect(out.provenance).toEqual(sampleDetail.provenance);
  });

  it("redacts followUp.verb when present", () => {
    const out = redactAuditDetail(sampleDetail, VIEWER_POLICY_DELEGATOR);
    expect(out.output.followUp?.verb).toContain(COUNTERPARTY_SENTINEL);
  });

  it("leaves detail untouched for an operator", () => {
    const out = redactAuditDetail(sampleDetail, VIEWER_POLICY_OPERATOR);
    expect(out.inputs.triggerSummary).toBe(sampleDetail.inputs.triggerSummary);
    expect(out.output.body).toBe(sampleDetail.output.body);
    expect(out.approver?.name).toBe("Petra Operator");
    expect(out.dataClassification).toBe("confidential");
  });
});

describe("per-category permissions", () => {
  it("auditor policy: keeps email/phone, masks card", () => {
    const policy: ViewerPolicy = {
      ...policyFromPermissions(ROLE_DEFAULT_PERMISSIONS.auditor),
      viewerRole: "auditor",
      permissions: ROLE_DEFAULT_PERMISSIONS.auditor,
      activeGrants: [],
    };
    const verb = "Sent sam@acmeco.com paid 4111 1111 1111 1111";
    const { redacted } = redactPII(verb, policy);
    expect(redacted).toContain("sam@acmeco.com"); // email preserved
    expect(redacted).toContain("[CARD]"); // card masked
  });

  it("compliance-officer policy: passes everything through", () => {
    const policy: ViewerPolicy = {
      ...policyFromPermissions(ROLE_DEFAULT_PERMISSIONS["compliance-officer"]),
      viewerRole: "compliance-officer",
      permissions: ROLE_DEFAULT_PERMISSIONS["compliance-officer"],
      activeGrants: [],
    };
    const out = redactAuditRow(sampleRow, policy);
    expect(out.verb).toBe(sampleRow.verb);
    expect(out.approver?.name).toBe("Petra Operator");
  });

  it("categoriesLackedBy: delegator lacks all 8", () => {
    expect(categoriesLackedBy(NO_PERMISSIONS)).toHaveLength(8);
  });

  it("categoriesLackedBy: operator lacks none", () => {
    expect(categoriesLackedBy(ALL_PERMISSIONS)).toEqual([]);
  });
});

describe("textHasRedaction", () => {
  it("detects pattern sentinels", () => {
    expect(textHasRedaction("ping [EMAIL] then")).toBe(true);
  });

  it("detects audit sentinels", () => {
    expect(textHasRedaction(`approved by ${APPROVER_SENTINEL}`)).toBe(true);
    expect(textHasRedaction(`via ${COUNTERPARTY_SENTINEL}`)).toBe(true);
  });

  it("returns false for clean text", () => {
    expect(textHasRedaction("ordinary log line, no sentinels")).toBe(false);
  });
});

describe("resolveViewerPolicy", () => {
  it("unredacted-view permission grants operator-equivalent", () => {
    const policy = resolveViewerPolicy(["unredacted-view"], "delegator", []);
    expect(policy.viewerRole).toBe("operator");
    expect(policy.permissions).toEqual(ALL_PERMISSIONS);
  });

  it("delegator with no grants and no perms returns the canonical all-mask policy", () => {
    const policy = resolveViewerPolicy([], "delegator", []);
    expect(policy).toEqual(VIEWER_POLICY_DELEGATOR);
  });

  it("per-permission grants flip the matching key on", () => {
    const policy = resolveViewerPolicy(["unredacted-email"], "delegator", []);
    expect(policy.permissions.unredactedEmail).toBe(true);
    expect(policy.permissions.unredactedPhone).toBe(false);
    expect(policy.email).toBe("show");
  });

  it("active elevated-view grants merge into permissions", () => {
    const policy = resolveViewerPolicy([], "delegator", [
      {
        grantedAt: new Date().toISOString(),
        expiresAt: new Date(Date.now() + 3600_000).toISOString(),
        categories: ["unredacted-counterparty"],
        reason: "Compliance review #CR-2024-08",
        requestId: "g_1",
      },
    ]);
    expect(policy.permissions.unredactedCounterparty).toBe(true);
    expect(policy.activeGrants).toHaveLength(1);
  });
});
