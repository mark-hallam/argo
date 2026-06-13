/**
 * Resolve a presentable, human role subtitle for a Worker.
 *
 * Shared by `WorkerHero` (the worker-profile header) and `TeammateCard`
 * (the /team + /workers deck) so the role label is derived identically
 * everywhere a teammate's role is shown.
 *
 * Two leaks this guards against, both surfaced by the 2026-06-13 E2E
 * shakedown (finding F13):
 *
 *  1. A fresh Genesis hire can carry a *provenance marker* in
 *     `role.tagline` — "Genesis <name> <plan_id>", e.g.
 *     "Genesis Rex 019ebea4d72d1b81". That is an audit/provenance
 *     string, not a job title; showing it under the teammate's name
 *     reads like a bug and the first-impression magic dips. The
 *     provenance stays available where it belongs (the Plan-provenance
 *     chip / audit view) — it just must never be the role subtitle.
 *  2. A bare lowercase slug — often the teammate's own name ("ivy") —
 *     leaking through `role.tagline`.
 *
 * Resolution order: a real human tagline (has a space or a capital and
 * is not a provenance marker) is shown as-is; otherwise fall back to a
 * humanized vertical ("collections" → "Collections"), then a humanized
 * slug, and finally a generic "Teammate" so the subtitle is never raw.
 */

import type { Worker } from "@/lib/types/worker";

/**
 * Matches a Genesis provenance marker such as "Genesis Rex
 * 019ebea4d72d1b81": the literal "Genesis" provenance word, the
 * teammate name, then a long hex plan id (≥12 hex chars). This is the
 * string Genesis stamps for provenance, NOT a human role title.
 */
const GENESIS_PROVENANCE_MARKER = /^genesis\b.*\b[0-9a-f]{12,}\b\.?$/i;

/** True when `tagline` is a Genesis provenance marker, not a role title. */
export function isGenesisProvenanceMarker(tagline: string): boolean {
  return GENESIS_PROVENANCE_MARKER.test(tagline.trim());
}

/** Turn a slug ("collections-coordinator") into Title Case ("Collections Coordinator"). */
export function humanizeSlug(slug: string): string {
  return slug
    .replace(/[-_]+/g, " ")
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

/** Resolve the user-facing role subtitle for a worker. Never returns a raw slug or provenance marker. */
export function resolveRoleLabel(worker: Worker): string {
  const rawTagline = worker.role.tagline?.trim() ?? "";
  // A Genesis provenance marker is never a role title — drop it and fall
  // through to the vertical-derived title.
  const tagline = rawTagline && !isGenesisProvenanceMarker(rawTagline) ? rawTagline : "";

  if (tagline && (/\s/.test(tagline) || /[A-Z]/.test(tagline))) return tagline;

  const vertical = worker.role.vertical?.trim() ?? "";
  const isNameSlug = tagline.toLowerCase() === worker.name.trim().toLowerCase();
  if (tagline && !isNameSlug) return humanizeSlug(tagline);
  if (vertical) return humanizeSlug(vertical);
  return "Teammate";
}
