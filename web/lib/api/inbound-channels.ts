/**
 * Inbound channels API client — recent operator replies that landed
 * on the SMS / Discord / Voice DTMF webhooks. Backed by
 * `GET /api/v1/inbound-channels/{kind}/recent` in
 * `crates/argo-cli/src/handlers/gateway/routes_inbound_channels.rs`.
 *
 * Returns an empty list when the gateway is unreachable or the route
 * is 404 (older gateway). Other ApiError statuses propagate so the
 * page can render a banner.
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { InboundKind } from "@/lib/notify-channels/inbound-registry";
import { isMissingOrUnreachable } from "./unreachable";

export interface InboundChannelEvent {
  seq: number;
  id: string;
  action_id: string | null;
  replier: string | null;
  decision: "approve" | "reject" | null;
  reply_body: string | null;
  replied_at: string | null;
  created_at_micros: number;
}

interface InboundChannelRecentResponse {
  kind: InboundKind;
  items: InboundChannelEvent[];
}

export async function fetchInboundChannelRecent(
  kind: InboundKind,
  limit = 20,
): Promise<InboundChannelEvent[]> {
  try {
    const res = await apiGet<InboundChannelRecentResponse>(
      `/api/v1/inbound-channels/${kind}/recent?limit=${limit}`,
    );
    return res.items;
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return [];
    }
    throw err;
  }
}
