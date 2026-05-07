export type Decision = "approve" | "reject";

export interface ApprovalRow {
  approval_id: string;
  action_id: string;
  risk_level: string;
  prompt_body: string;
  expiry: string;
  enqueued_at: string;
}

export interface InboxSnapshot {
  type: "snapshot";
  events: Array<{
    type: "approval_pending";
    approval_id: string;
    action_id: string;
    risk_level: string;
  }>;
}

export interface AppSettings {
  gateway_url: string;
  bearer_token: string;
}

export const SETTINGS_DEFAULTS: AppSettings = {
  gateway_url: "http://localhost:7090",
  bearer_token: "",
};
