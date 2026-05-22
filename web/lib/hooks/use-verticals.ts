"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchVertical,
  fetchVerticals,
  type FetchVerticalResult,
  type FetchVerticalsResult,
} from "@/lib/api/verticals";

const VERTICALS_KEY = ["verticals"] as const;

/**
 * Marketplace listing of registered vertical-templates. Falls back
 * to a seed catalog in dev when the gateway is unreachable; strict
 * production propagates the network error so the operator sees it.
 */
export function useVerticals() {
  return useQuery<FetchVerticalsResult>({
    queryKey: VERTICALS_KEY,
    queryFn: fetchVerticals,
  });
}

/** Single vertical-template detail; backs `/verticals/<id>`. */
export function useVertical(id: string | undefined) {
  return useQuery<FetchVerticalResult>({
    queryKey: ["vertical", id ?? ""],
    queryFn: () => fetchVertical(id as string),
    enabled: typeof id === "string" && id.length > 0,
  });
}
