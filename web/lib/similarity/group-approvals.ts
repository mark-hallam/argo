/**
 * Similarity grouping for the approvals inbox (Stream E1-d).
 *
 * Detects clusters of approvals that share `worker + action_type +
 * normalized-detail`. When ≥ `SIMILARITY_THRESHOLD` items share a
 * cluster, the surface above them renders a batch affordance.
 *
 * Detection is intentionally fuzzy and rule-based — good enough for
 * v1, easy to test, no model dependency. LLM-based similarity is a
 * v1.5 enhancement; do not add it here.
 */

import type { ApprovalItem } from "@/lib/sample-approvals";

/**
 * Minimum cluster size to form a group. Below this threshold the
 * items stay ungrouped and render as plain cards.
 *
 * Bumping this to 4 makes the surface quieter; dropping to 2 makes
 * it noisier (every duplicate triggers a banner). 3 is the journey-
 * doc-cited "approve all 8 in a row" floor.
 */
export const SIMILARITY_THRESHOLD = 3;

export interface ApprovalGroup {
  /** Stable hash key. Same for all items in the cluster. */
  key: string;
  /** All items in a group share the same worker. */
  worker: { name: string; avatarUrl?: string };
  /** Action-type bucket — drives noun selection. */
  actionType: string;
  /** Cluster members, in input order. */
  items: ApprovalItem[];
  /** Human-readable suffix, e.g. "5 similar replies". */
  reason: string;
}

/**
 * A flat ordered list of render units. Group members are emitted as
 * a single `group` unit at the position of the cluster's first item
 * — this keeps group members contiguous (per brief: no interspersed
 * ungrouped cards) while preserving recency order for everything
 * else.
 */
export type RenderUnit =
  | { kind: "single"; item: ApprovalItem }
  | { kind: "group"; group: ApprovalGroup };

/**
 * Normalize a free-form detail string into a comparable token bag.
 * Tokens < 4 chars are dropped (stop-word filter), the rest are
 * sorted + joined. Two details that differ only in stop-words,
 * names, or numbers collapse to the same hash.
 */
export function normalizeDetail(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^\w\s]/g, " ")
    .split(/\s+/)
    .filter((t) => t.length > 3)
    .sort()
    .join(" ");
}

/**
 * Action-type → [singular, plural] noun. Keys are derived from
 * `actionTypeOf(verb)`; add a new key when a new vertical lands.
 * Unknown action types fall back to "action(s)".
 */
const ACTION_NOUN: Record<string, [string, string]> = {
  "marketplace-reply": ["reply", "replies"],
  "marketplace-counter": ["counter-offer", "counter-offers"],
  "calendar-book": ["interview", "interviews"],
  "refund-issue": ["refund", "refunds"],
  "broadcast-publish": ["broadcast", "broadcasts"],
  "engineering-escalate": ["escalation", "escalations"],
  "candidate-reject": ["rejection", "rejections"],
};

const FALLBACK_NOUN: [string, string] = ["action", "actions"];

export function actionNoun(actionType: string): [string, string] {
  return ACTION_NOUN[actionType] ?? FALLBACK_NOUN;
}

/**
 * Pattern-based classifier: verb string → action-type bucket. Order
 * matters — earlier patterns win (e.g. "counter-offer" beats
 * "marketplace"). Unknown verbs fall through to a normalized-verb
 * bucket so similar verbs still cluster, even without a friendly
 * noun.
 */
const VERB_PATTERNS: Array<[RegExp, string]> = [
  [/counter[- ]offer/i, "marketplace-counter"],
  [/listing|marketplace/i, "marketplace-reply"],
  [/interview|\bbook\b/i, "calendar-book"],
  [/refund/i, "refund-issue"],
  [/publish|broadcast|announce/i, "broadcast-publish"],
  [/escalat/i, "engineering-escalate"],
  [/reject/i, "candidate-reject"],
];

export function actionTypeOf(verb: string): string {
  for (const [pat, type] of VERB_PATTERNS) {
    if (pat.test(verb)) return type;
  }
  const fallback = verb
    .toLowerCase()
    .split(/\s+/)
    .filter((t) => t.length > 3)
    .sort()
    .join(" ");
  return fallback || "action";
}

function similarityKey(item: ApprovalItem): string {
  return `${item.worker.name}|${actionTypeOf(item.verb)}|${normalizeDetail(item.detail)}`;
}

/**
 * Group approvals by similarity. Returns:
 *  - `units`: flat ordered list for rendering (groups contiguous).
 *  - `groups`: cluster summaries (≥ threshold members).
 *  - `ungrouped`: items that didn't reach the threshold.
 *
 * Pure function; safe to memoize on the input array reference.
 */
export function groupApprovals(items: ApprovalItem[]): {
  units: RenderUnit[];
  groups: ApprovalGroup[];
  ungrouped: ApprovalItem[];
} {
  const buckets = new Map<string, ApprovalItem[]>();
  for (const item of items) {
    const k = similarityKey(item);
    const arr = buckets.get(k);
    if (arr) arr.push(item);
    else buckets.set(k, [item]);
  }

  const groups: ApprovalGroup[] = [];
  const groupByKey = new Map<string, ApprovalGroup>();
  const ungrouped: ApprovalItem[] = [];

  for (const [key, members] of buckets) {
    if (members.length >= SIMILARITY_THRESHOLD) {
      const first = members[0];
      const actionType = actionTypeOf(first.verb);
      const [, plural] = actionNoun(actionType);
      const group: ApprovalGroup = {
        key,
        worker: first.worker,
        actionType,
        items: members,
        reason: `${members.length} similar ${plural}`,
      };
      groups.push(group);
      groupByKey.set(key, group);
    } else {
      ungrouped.push(...members);
    }
  }

  const emittedGroups = new Set<string>();
  const units: RenderUnit[] = [];
  for (const item of items) {
    const k = similarityKey(item);
    const g = groupByKey.get(k);
    if (g) {
      if (!emittedGroups.has(k)) {
        emittedGroups.add(k);
        units.push({ kind: "group", group: g });
      }
    } else {
      units.push({ kind: "single", item });
    }
  }

  return { units, groups, ungrouped };
}
