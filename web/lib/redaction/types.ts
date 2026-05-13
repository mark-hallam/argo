/**
 * RBAC-aware redaction layer for audit surfaces (E5-g). Extends the
 * E1-e base with per-category permissions, viewer roles, and
 * temporary elevated-view grants. v1 enforcement is render-time /
 * client-side; the server-side first-fence is sibling brief
 * `mission-enterprise-audit-pii-server-fence`.
 */

import {
  POLICY_MASK_ALL,
  POLICY_SHOW_ALL,
  type RedactionPolicy,
} from "./redact";

/** RBAC permissions that grant per-category unredacted views. */
export type RbacPermission =
  | "unredacted-email"
  | "unredacted-phone"
  | "unredacted-card"
  | "unredacted-iban"
  | "unredacted-ird"
  | "unredacted-classification"
  | "unredacted-approver"
  | "unredacted-counterparty";

export const RBAC_PERMISSIONS = [
  "unredacted-email", "unredacted-phone", "unredacted-card",
  "unredacted-iban", "unredacted-ird", "unredacted-classification",
  "unredacted-approver", "unredacted-counterparty",
] as const satisfies readonly RbacPermission[];

/** Per-category boolean view of `RbacPermission[]`. Keyed in the
 *  shape `unredactedX`, matching the brief's `ViewerPolicy` example. */
export interface ViewerPermissions {
  unredactedEmail: boolean;
  unredactedPhone: boolean;
  unredactedCard: boolean;
  unredactedIBAN: boolean;
  unredactedIRD: boolean;
  unredactedClassification: boolean;
  unredactedApprover: boolean;
  unredactedCounterparty: boolean;
}

/** Viewer roles whose default permissions are wired by `useViewerPolicy`. */
export type ViewerRole = "delegator" | "operator" | "auditor" | "compliance-officer";

export const VIEWER_ROLES: readonly ViewerRole[] = [
  "delegator",
  "operator",
  "auditor",
  "compliance-officer",
] as const;

/** A temporary unlock — issued client-side in v1, server-bound in v1.5. */
export interface ElevatedGrant {
  grantedAt: string;
  expiresAt: string;
  categories: RbacPermission[];
  reason: string;
  requestId: string;
}

/** Audit-surface policy — combines the E1-e pattern policy with
 *  category permissions, role, and active elevated grants. */
export interface ViewerPolicy extends RedactionPolicy {
  viewerRole: ViewerRole;
  permissions: ViewerPermissions;
  activeGrants: ElevatedGrant[];
}

function uniformPermissions(value: boolean): ViewerPermissions {
  return {
    unredactedEmail: value,
    unredactedPhone: value,
    unredactedCard: value,
    unredactedIBAN: value,
    unredactedIRD: value,
    unredactedClassification: value,
    unredactedApprover: value,
    unredactedCounterparty: value,
  };
}

export const NO_PERMISSIONS = uniformPermissions(false);
export const ALL_PERMISSIONS = uniformPermissions(true);

/** Default permissions per role. Auditor extends the brief's
 *  spec (Email/Phone/Approver true; Card/IBAN/IRD false) with
 *  Classification + Counterparty true — auditors typically need
 *  both. See PR "Sensible deviations". */
export const ROLE_DEFAULT_PERMISSIONS: Record<ViewerRole, ViewerPermissions> = {
  delegator: { ...NO_PERMISSIONS },
  operator: { ...ALL_PERMISSIONS },
  auditor: {
    unredactedEmail: true,
    unredactedPhone: true,
    unredactedCard: false,
    unredactedIBAN: false,
    unredactedIRD: false,
    unredactedClassification: true,
    unredactedApprover: true,
    unredactedCounterparty: true,
  },
  "compliance-officer": { ...ALL_PERMISSIONS },
};

/** Map between the RBAC string and the `ViewerPermissions` key. */
export const PERMISSION_KEY: Record<RbacPermission, keyof ViewerPermissions> = {
  "unredacted-email": "unredactedEmail",
  "unredacted-phone": "unredactedPhone",
  "unredacted-card": "unredactedCard",
  "unredacted-iban": "unredactedIBAN",
  "unredacted-ird": "unredactedIRD",
  "unredacted-classification": "unredactedClassification",
  "unredacted-approver": "unredactedApprover",
  "unredacted-counterparty": "unredactedCounterparty",
};

/** Build a `RedactionPolicy` from `ViewerPermissions` — masks any
 *  pattern category that the viewer is NOT permitted to see. */
export function policyFromPermissions(perms: ViewerPermissions): RedactionPolicy {
  return {
    email: perms.unredactedEmail ? "show" : "mask",
    phone: perms.unredactedPhone ? "show" : "mask",
    card: perms.unredactedCard ? "show" : "mask",
    iban: perms.unredactedIBAN ? "show" : "mask",
    ird: perms.unredactedIRD ? "show" : "mask",
  };
}

/** All-mask viewer policy — a delegator with no grants. */
export const VIEWER_POLICY_DELEGATOR: ViewerPolicy = {
  ...POLICY_MASK_ALL,
  viewerRole: "delegator",
  permissions: { ...NO_PERMISSIONS },
  activeGrants: [],
};

/** All-show viewer policy — for tests / dev bypass. */
export const VIEWER_POLICY_OPERATOR: ViewerPolicy = {
  ...POLICY_SHOW_ALL,
  viewerRole: "operator",
  permissions: { ...ALL_PERMISSIONS },
  activeGrants: [],
};
