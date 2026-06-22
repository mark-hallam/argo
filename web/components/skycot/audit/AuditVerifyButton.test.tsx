import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AuditVerifyButton } from "./AuditVerifyButton";
import type { AuditVerifyResult } from "@/lib/api/audit-verify";

const verifyAuditChain = vi.hoisted(() => vi.fn());
vi.mock("@/lib/api/audit-verify", () => ({ verifyAuditChain }));

function result(overrides: Partial<AuditVerifyResult> = {}): AuditVerifyResult {
  return {
    intact: true,
    rows_checked: 42,
    breaks: 0,
    first_broken_seq: null,
    head_hash: "ab12cd34ef56ab12cd34ef56ab12cd34ef56ab12cd34ef56ab12cd34ef560011",
    verified_at: "2026-06-22T04:00:00Z",
    tenant_id: "op-x",
    ...overrides,
  };
}

afterEach(() => {
  verifyAuditChain.mockReset();
});

describe("AuditVerifyButton", () => {
  it("starts idle with a Verify integrity affordance (no claim made before a check runs)", () => {
    render(<AuditVerifyButton />);
    expect(screen.getByTestId("audit-verify-button")).toHaveTextContent(/verify integrity/i);
    expect(screen.queryByTestId("audit-verify-result")).toBeNull();
  });

  it("on an intact chain, reports the REAL verified state (rows + head hash)", async () => {
    const user = userEvent.setup();
    verifyAuditChain.mockResolvedValue(result());
    render(<AuditVerifyButton />);

    await user.click(screen.getByTestId("audit-verify-button"));

    const out = await screen.findByTestId("audit-verify-result");
    expect(out).toHaveAttribute("data-intact", "true");
    expect(out).toHaveTextContent(/Chain intact — 42 rows verified/);
    // The head hash is shown abbreviated (proof the result is real, not copy).
    expect(out).toHaveTextContent(/head ab12cd34ef/);
    expect(verifyAuditChain).toHaveBeenCalledTimes(1);
  });

  it("TRUTH-BOUNDED: a tampered chain shows BROKEN + the first offending row — never a green tick", async () => {
    const user = userEvent.setup();
    verifyAuditChain.mockResolvedValue(
      result({ intact: false, breaks: 2, first_broken_seq: 17, rows_checked: 42 }),
    );
    render(<AuditVerifyButton />);

    await user.click(screen.getByTestId("audit-verify-button"));

    const out = await screen.findByTestId("audit-verify-result");
    expect(out).toHaveAttribute("data-intact", "false");
    expect(out).toHaveTextContent(/Chain BROKEN at row 17/);
    expect(out).toHaveTextContent(/2 breaks/);
  });

  it("on error (e.g. 401 / gateway down) says it couldn't verify rather than implying success", async () => {
    const user = userEvent.setup();
    verifyAuditChain.mockRejectedValue(new Error("unauthorized"));
    render(<AuditVerifyButton />);

    await user.click(screen.getByTestId("audit-verify-button"));

    const err = await screen.findByTestId("audit-verify-error");
    expect(err).toHaveTextContent(/couldn't verify/i);
    expect(err).toHaveTextContent(/unauthorized/);
    // No success surface is ever shown on failure.
    expect(screen.queryByTestId("audit-verify-result")).toBeNull();
  });

  it("can re-verify after a result", async () => {
    const user = userEvent.setup();
    verifyAuditChain.mockResolvedValue(result());
    render(<AuditVerifyButton />);

    await user.click(screen.getByTestId("audit-verify-button"));
    await screen.findByTestId("audit-verify-result");
    await user.click(screen.getByRole("button", { name: /re-verify/i }));

    await waitFor(() => expect(verifyAuditChain).toHaveBeenCalledTimes(2));
  });
});
