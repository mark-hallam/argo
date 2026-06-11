/**
 * Genesis hire-draft intake helpers (UF Phase-A · DEC-025 #7).
 *
 * Sibling of `genesis.ts` (which is grandfathered over the 500-LOC ceiling).
 * This module owns the hire-draft builder + the pre-hire "review and edit"
 * merge so the operator can correct the captured cadence / exceptions /
 * success-criteria on the review screen before confirming the hire.
 *
 * Edits are ADVISORY: empty / skipped fields fall back to the server echo
 * (`vertical_ready.role_contract`, the single source of truth), and none of
 * these three slots is part of the governance `contract_digest`, so editing
 * them never invalidates the operator's confirmation — the Confirm button
 * stays enabled. The `genesis.ts` hire path re-imports `buildHireDraftFromGenesis`
 * from here unchanged.
 *
 * Type-only imports from `./genesis` are erased at compile time, so there is no
 * runtime import cycle (the value import goes the other way: genesis.ts → here).
 */

import { parseCadenceToCron } from "./genesis-cadence";
import type {
  GenesisVerticalReadyEvent,
  RoleContract,
} from "./genesis";
import type { CadenceExceptionsSelection } from "@/components/skycot/genesis/CadenceExceptionsCapture";

/**
 * Seed a `CadenceExceptionsSelection` from the server's role contract so the
 * pre-hire editor opens pre-filled with what the teammate was actually
 * generated with. Missing fields become the empty selection.
 */
export function intakeFromContract(
  contract: RoleContract | null | undefined,
): CadenceExceptionsSelection {
  return {
    cadence: contract?.cadence ?? "",
    exceptions: contract?.exceptions ? [...contract.exceptions] : [],
    success_criteria: contract?.success_criteria
      ? [...contract.success_criteria]
      : [],
  };
}

/**
 * Merge the operator's pre-hire edits back into the streamed
 * `vertical_ready` event's role contract. Empty / skipped edits fall back to
 * the server-echoed values (no client-side divergence). Returns a new event;
 * `contract_digest` and every other field are preserved, so the hire
 * confirmation still matches.
 */
export function applyIntakeEdits(
  ready: GenesisVerticalReadyEvent,
  edits: CadenceExceptionsSelection,
): GenesisVerticalReadyEvent {
  const base = ready.role_contract;
  if (!base) return ready;
  const cadence =
    edits.cadence.trim().length > 0 ? edits.cadence.trim() : base.cadence ?? null;
  const exceptions =
    edits.exceptions.length > 0 ? edits.exceptions : base.exceptions ?? [];
  const success_criteria =
    edits.success_criteria.length > 0
      ? edits.success_criteria
      : base.success_criteria ?? [];
  return {
    ...ready,
    role_contract: { ...base, cadence, exceptions, success_criteria },
  };
}

/**
 * Best-effort: pull a local folder path out of the captured Access needs so
 * the hire can bind `worker.fs_root` to the folder the user actually NAMED at
 * intake (uf-bind-intake-access-folder-to-fs-root). Returns the path-ish token
 * of the FIRST access need that denotes a local folder, or `undefined` when
 * none does — in which case the hire keeps the generated Genesis default.
 *
 * This is a CONVENIENCE extractor, NOT the security boundary: the gateway
 * re-validates the candidate (must resolve under the user's HOME, no `..`
 * traversal) before binding it (`argo_identity::safe_access_fs_root`). A vague
 * or non-path answer (a bare integration name like "Gmail", or a full
 * sentence) yields `undefined` so the server falls back to the default.
 */
export function extractFsRootFromAccessNeeds(
  accessNeeds: string[] | undefined,
): string | undefined {
  if (!accessNeeds) return undefined;
  for (const raw of accessNeeds) {
    const candidate = folderCandidate(raw);
    if (candidate) return candidate;
  }
  return undefined;
}

/**
 * Reduce one Access-need string to a local folder path token, or `undefined`.
 * Rule: strip a trailing descriptor word ("folder"/"directory"/"dir"), then
 * accept the remainder only when it is a SINGLE whitespace-free token that
 * looks like a path — it starts with `~` or `/`, or contains a `/` segment.
 * A bare integration name ("Gmail") or a multi-word phrase is rejected.
 */
function folderCandidate(raw: string): string | undefined {
  const stripped = raw.trim().replace(/\s+(folder|directory|dir)\b\.?$/i, "").trim();
  if (!stripped || /\s/.test(stripped)) return undefined;
  const looksLikePath =
    stripped.startsWith("~") || stripped.startsWith("/") || stripped.includes("/");
  return looksLikePath ? stripped : undefined;
}

/**
 * Build a canonical `HireDraft` (snake_case → wire camelCase) from the
 * SSE `vertical_ready` payload. The Rust validator requires:
 *
 * - `suggestedName` non-empty
 * - `charter` >= 20 chars
 * - `suggestedVertical` non-empty + resolvable in the catalog (or
 *   `.genesis/<tenant>/`)
 * - at least one duty with `defaultEnabled = true`
 * - `approvalPolicy.level` in {conservative, balanced, aggressive}
 *
 * When the role contract carries operator-captured (or operator-EDITED, via
 * [[applyIntakeEdits]]) cadence / exceptions / success-criteria, they ride
 * along on the draft. The gateway's `HireDraft` deserializer ignores unknown
 * fields (no `deny_unknown_fields`), so these advisory slots are forward-safe.
 */
export function buildHireDraftFromGenesis(
  ready: GenesisVerticalReadyEvent,
  roleDescription: string,
): Record<string, unknown> {
  const name = (ready.manifest.name || "Operator").trim();
  // Charter has to clear the 20-char floor; pad with the role description
  // so we never trip the validator on a terse manifest name.
  const charter = [
    `${name} owns this work end-to-end.`,
    roleDescription.trim(),
  ]
    .join("\n\n")
    .slice(0, 4000);
  // DEC-048 rule 2 (captured-equals-built): the ONE seed duty fires WHEN the
  // operator asked at intake (or as EDITED via [[applyIntakeEdits]]), not on a
  // hardcoded default. Its description is the operator's OWN cadence words
  // (never canned copy), and its cron is the deterministic parse of those
  // words. An unparseable cadence keeps the words + falls back to the default
  // cron and is flagged "needs confirmation" on the review card — never
  // silently substituted. (Ported verbatim from #2798's in-place version when
  // this function moved here; the relocation must not revert that P1 fix.)
  const cadenceWords = ready.role_contract?.cadence?.trim();
  const { cron } = parseCadenceToCron(cadenceWords);
  const description =
    cadenceWords && cadenceWords.length > 0
      ? cadenceWords.slice(0, 280)
      : "Runs on a default weekday schedule until you set one in Schedules.";
  const duties = [
    {
      cron,
      entryAction: "respond_to_inbound",
      description,
      defaultEnabled: true,
    },
  ];
  const draft: Record<string, unknown> = {
    suggestedName: name,
    charter,
    suggestedVertical: ready.vertical_id,
    approvalPolicy: { level: "balanced" },
    duties,
    tagline: ready.manifest.persona || ready.manifest.archetype,
    // DEC-025: carry the Genesis plan id so the persisted worker gets a
    // `genesis_plan_id=` provenance marker. This is load-bearing for
    // SAFETY, not just the provenance chip: the runtime capability fence
    // (P2) AND the enforced probation window (P3) both key off this
    // marker. Without it the flagship SSE-hired teammate would be hired
    // UNFENCED (owner `*` keys) and with NO probation — strictly weaker
    // than the `/onboarding/first-hire` path, which already sets it.
    genesisPlanId: ready.vertical_id,
  };
  // Advisory intake slots (DEC-025 #7) — carried only when present so the
  // wire shape is byte-unchanged for a teammate with none captured.
  const rc = ready.role_contract;
  if (rc) {
    if (rc.cadence && rc.cadence.trim().length > 0) draft.cadence = rc.cadence;
    if (rc.exceptions && rc.exceptions.length > 0) draft.exceptions = rc.exceptions;
    if (rc.success_criteria && rc.success_criteria.length > 0) {
      draft.successCriteria = rc.success_criteria;
    }
    // uf-bind-intake-access-folder-to-fs-root: when the captured Access needs
    // name a local folder, carry it so the gateway can bind worker.fs_root to
    // the folder the user actually named (server re-validates under HOME).
    const fsRoot = extractFsRootFromAccessNeeds(rc.access_needs);
    if (fsRoot) draft.accessFsRoot = fsRoot;
  }
  return draft;
}
