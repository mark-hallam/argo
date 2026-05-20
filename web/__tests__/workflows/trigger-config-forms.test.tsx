import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import WebhookConfigForm from "@/components/workflows/triggers/WebhookConfigForm";
import ScheduleConfigForm, {
  buildCron,
  explainSchedule,
  nextNRuns,
} from "@/components/workflows/triggers/ScheduleConfigForm";
import EmailConfigForm from "@/components/workflows/triggers/EmailConfigForm";
import FormConfigForm from "@/components/workflows/triggers/FormConfigForm";
import TriggerConfigDrawer from "@/components/workflows/triggers/TriggerConfigDrawer";
import {
  defaultEmailConfig,
  defaultFormConfig,
  defaultScheduleConfig,
  defaultWebhookConfig,
  webhookConfigSchema,
  scheduleConfigSchema,
  formConfigSchema,
  type FormConfig,
  type EmailConfig,
  type ScheduleConfig,
  type WebhookConfig,
} from "@/components/workflows/triggers/types";

function WebhookHarness({ initial }: { initial: WebhookConfig }) {
  const [cfg, setCfg] = useState(initial);
  return <WebhookConfigForm config={cfg} onChange={setCfg} />;
}

function ScheduleHarness({ initial }: { initial: ScheduleConfig }) {
  const [cfg, setCfg] = useState(initial);
  return <ScheduleConfigForm config={cfg} onChange={setCfg} />;
}

function EmailHarness({ initial, onTest }: { initial: EmailConfig; onTest?: (c: EmailConfig) => Promise<{ ok: boolean; message: string }> }) {
  const [cfg, setCfg] = useState(initial);
  return <EmailConfigForm config={cfg} onChange={setCfg} onTestConnection={onTest} />;
}

function FormHarness({ initial }: { initial: FormConfig }) {
  const [cfg, setCfg] = useState(initial);
  return <FormConfigForm config={cfg} onChange={setCfg} />;
}

describe("WebhookConfigForm", () => {
  it("renders generated URL with workflow id + secret + copy button + regenerate", () => {
    const cfg = defaultWebhookConfig("wf-123");
    render(<WebhookHarness initial={cfg} />);

    const urlInput = screen.getByTestId("webhook-url") as HTMLInputElement;
    expect(urlInput.value).toMatch(/^https:\/\/api\.skycot\.com\/v1\/webhooks\/wf-123\/[a-z0-9]{32}$/);
    expect(screen.getByTestId("webhook-copy")).toBeInTheDocument();
    expect(screen.getByTestId("webhook-regenerate")).toBeInTheDocument();
  });

  it("rotates the secret after a confirm click", async () => {
    const user = userEvent.setup();
    const cfg = defaultWebhookConfig("wf-x");
    render(<WebhookHarness initial={cfg} />);
    const before = (screen.getByTestId("webhook-url") as HTMLInputElement).value;

    await user.click(screen.getByTestId("webhook-regenerate"));
    // first click switches to confirm mode, does not rotate
    expect((screen.getByTestId("webhook-url") as HTMLInputElement).value).toBe(before);
    await user.click(screen.getByTestId("webhook-regenerate"));
    expect((screen.getByTestId("webhook-url") as HTMLInputElement).value).not.toBe(before);
  });

  it("test-ping calls injected handler and shows status", async () => {
    const user = userEvent.setup();
    const cfg = defaultWebhookConfig("wf-x");
    const ping = vi.fn().mockResolvedValue({ status: 200, body: '{"ok":true}' });
    function Harness() {
      const [c, setC] = useState(cfg);
      return <WebhookConfigForm config={c} onChange={setC} onTestPing={ping} />;
    }
    render(<Harness />);
    await user.click(screen.getByTestId("webhook-test-ping"));
    await waitFor(() => expect(ping).toHaveBeenCalled());
    expect(screen.getByTestId("webhook-ping-status").textContent).toMatch(/HTTP 200/);
    expect(screen.getByTestId("webhook-ping-body")).toHaveTextContent('{"ok":true}');
  });

  it("zod schema rejects empty allowedMethods", () => {
    const parsed = webhookConfigSchema.safeParse({ ...defaultWebhookConfig("w"), allowedMethods: [] });
    expect(parsed.success).toBe(false);
  });

  it("zod schema rejects header-auth missing name/value", () => {
    const parsed = webhookConfigSchema.safeParse({ ...defaultWebhookConfig("w"), authMode: "header" });
    expect(parsed.success).toBe(false);
  });
});

describe("ScheduleConfigForm", () => {
  it("renders frequency pills + timezone + next-runs preview", () => {
    render(<ScheduleHarness initial={defaultScheduleConfig("Australia/Melbourne")} />);
    expect(screen.getByText("Every day")).toBeInTheDocument();
    expect(screen.getByTestId("schedule-timezone")).toBeInTheDocument();
    const items = screen.getByTestId("schedule-next-runs").querySelectorAll("li");
    expect(items.length).toBe(5);
  });

  it("explains the schedule in plain English", () => {
    render(<ScheduleHarness initial={defaultScheduleConfig("Australia/Melbourne")} />);
    expect(screen.getByTestId("schedule-summary").textContent).toMatch(/Australia\/Melbourne/);
  });

  it("buildCron renders cron syntax per mode", () => {
    expect(buildCron({ kind: "schedule", mode: "minute", timezone: "UTC" })).toBe("* * * * *");
    expect(buildCron({ kind: "schedule", mode: "hour", minute: 30, timezone: "UTC" })).toBe("30 * * * *");
    expect(buildCron({ kind: "schedule", mode: "day", hour: 9, minute: 0, timezone: "UTC" })).toBe("0 9 * * *");
    expect(buildCron({ kind: "schedule", mode: "week", hour: 9, minute: 0, daysOfWeek: [1, 3, 5], timezone: "UTC" })).toBe("0 9 * * 1,3,5");
    expect(buildCron({ kind: "schedule", mode: "month", hour: 12, minute: 0, dayOfMonth: 15, timezone: "UTC" })).toBe("0 12 15 * *");
  });

  it("nextNRuns returns 5 sequential daily timestamps for day mode", () => {
    const from = new Date("2026-05-20T00:00:00Z");
    const runs = nextNRuns(
      { kind: "schedule", mode: "day", hour: 9, minute: 0, timezone: "UTC" },
      5,
      from,
    );
    expect(runs).toHaveLength(5);
    // Each consecutive run should be roughly 24h apart (within DST tolerance).
    for (let i = 1; i < runs.length; i++) {
      const a = new Date(runs[i - 1]).getTime();
      const b = new Date(runs[i]).getTime();
      const delta = b - a;
      expect(delta).toBeGreaterThan(22 * 3600 * 1000);
      expect(delta).toBeLessThan(26 * 3600 * 1000);
    }
  });

  it("explainSchedule uses the configured timezone", () => {
    const s = explainSchedule({ kind: "schedule", mode: "day", hour: 9, minute: 0, timezone: "Europe/London" });
    expect(s).toMatch(/Europe\/London/);
  });

  it("zod schema rejects week mode with no days selected", () => {
    const parsed = scheduleConfigSchema.safeParse({ kind: "schedule", mode: "week", hour: 9, minute: 0, daysOfWeek: [], timezone: "UTC" });
    expect(parsed.success).toBe(false);
  });

  it("zod schema rejects custom mode with empty cron expression", () => {
    const parsed = scheduleConfigSchema.safeParse({ kind: "schedule", mode: "custom", cronExpression: "", timezone: "UTC" });
    expect(parsed.success).toBe(false);
  });
});

describe("EmailConfigForm", () => {
  it("renders IMAP credential fields", () => {
    render(<EmailHarness initial={defaultEmailConfig()} />);
    expect(screen.getByLabelText(/server/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^port$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/username/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^password$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/use ssl/i)).toBeInTheDocument();
  });

  it("test-connection calls injected handler and surfaces success", async () => {
    const user = userEvent.setup();
    const handler = vi.fn().mockResolvedValue({ ok: true, message: "Connected." });
    render(<EmailHarness initial={defaultEmailConfig()} onTest={handler} />);
    await user.click(screen.getByTestId("email-test-connection"));
    await waitFor(() => expect(handler).toHaveBeenCalled());
    expect(screen.getByTestId("email-test-result").textContent).toMatch(/Connected/);
  });

  it("falls back to coming-soon notice when no handler is provided", async () => {
    const user = userEvent.setup();
    render(<EmailHarness initial={defaultEmailConfig()} />);
    await user.click(screen.getByTestId("email-test-connection"));
    expect(screen.getByTestId("email-test-result").textContent).toMatch(/coming in v1\.x/i);
  });
});

describe("FormConfigForm", () => {
  it("renders form URL + visibility toggle + initial fields + submit label + success message", () => {
    render(<FormHarness initial={defaultFormConfig("wf-7")} />);
    expect((screen.getByTestId("form-url") as HTMLInputElement).value).toBe("https://skycot.com/forms/wf-7");
    expect(screen.getByText(/Public — anyone/)).toBeInTheDocument();
    expect(screen.getByText(/Private — SKYCOT login/)).toBeInTheDocument();
    expect(screen.getByDisplayValue("Submit")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Thanks! We'll be in touch.")).toBeInTheDocument();
  });

  it("adds and removes fields", async () => {
    const user = userEvent.setup();
    render(<FormHarness initial={defaultFormConfig("wf-7")} />);
    const list = () => screen.getByTestId("form-field-list");
    const before = list().querySelectorAll('[data-testid^="form-field-row-"]').length;
    expect(before).toBeGreaterThanOrEqual(2);

    await user.click(screen.getByTestId("form-add-field"));
    expect(list().querySelectorAll('[data-testid^="form-field-row-"]').length).toBe(before + 1);

    const deletes = screen.getAllByLabelText(/delete field/i);
    await user.click(deletes[deletes.length - 1]);
    expect(list().querySelectorAll('[data-testid^="form-field-row-"]').length).toBe(before);
  });

  it("zod schema rejects empty field list", () => {
    const parsed = formConfigSchema.safeParse({ ...defaultFormConfig("w"), fields: [] });
    expect(parsed.success).toBe(false);
  });
});

describe("TriggerConfigDrawer dispatcher", () => {
  it("renders webhook form for kind=webhook", async () => {
    function Wrapper() {
      const [c, setC] = useState<WebhookConfig | null>(null);
      return (
        <TriggerConfigDrawer
          triggerKind="webhook"
          workflowId="wf-1"
          workspaceTimezone="UTC"
          config={c}
          onChange={(next) => setC(next as WebhookConfig)}
          onClose={() => undefined}
        />
      );
    }
    render(<Wrapper />);
    await waitFor(() => expect(screen.getByTestId("webhook-config-form")).toBeInTheDocument());
  });

  it("renders schedule form for kind=schedule", async () => {
    function Wrapper() {
      const [c, setC] = useState<ScheduleConfig | null>(null);
      return (
        <TriggerConfigDrawer
          triggerKind="schedule"
          workflowId="wf-1"
          workspaceTimezone="Australia/Melbourne"
          config={c}
          onChange={(next) => setC(next as ScheduleConfig)}
          onClose={() => undefined}
        />
      );
    }
    render(<Wrapper />);
    await waitFor(() => expect(screen.getByTestId("schedule-config-form")).toBeInTheDocument());
  });

  it("renders email form for kind=email", async () => {
    function Wrapper() {
      const [c, setC] = useState<EmailConfig | null>(null);
      return (
        <TriggerConfigDrawer
          triggerKind="email"
          workflowId="wf-1"
          workspaceTimezone="UTC"
          config={c}
          onChange={(next) => setC(next as EmailConfig)}
          onClose={() => undefined}
        />
      );
    }
    render(<Wrapper />);
    await waitFor(() => expect(screen.getByTestId("email-config-form")).toBeInTheDocument());
  });

  it("renders form builder for kind=form", async () => {
    function Wrapper() {
      const [c, setC] = useState<FormConfig | null>(null);
      return (
        <TriggerConfigDrawer
          triggerKind="form"
          workflowId="wf-1"
          workspaceTimezone="UTC"
          config={c}
          onChange={(next) => setC(next as FormConfig)}
          onClose={() => undefined}
        />
      );
    }
    render(<Wrapper />);
    await waitFor(() => expect(screen.getByTestId("form-config-form")).toBeInTheDocument());
  });

  it("renders no-extra-config message for manual + slack", () => {
    render(
      <TriggerConfigDrawer
        triggerKind="manual"
        workflowId="wf-1"
        workspaceTimezone="UTC"
        config={null}
        onChange={() => undefined}
        onClose={() => undefined}
      />,
    );
    expect(screen.getByTestId("trigger-no-extra-config-manual")).toBeInTheDocument();
  });

  it("close button calls onClose", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <TriggerConfigDrawer
        triggerKind="manual"
        workflowId="wf-1"
        workspaceTimezone="UTC"
        config={null}
        onChange={() => undefined}
        onClose={onClose}
      />,
    );
    await user.click(screen.getByLabelText(/close trigger config/i));
    expect(onClose).toHaveBeenCalled();
  });
});
