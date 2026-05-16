"use client";

import { useMutation } from "@tanstack/react-query";

export interface CreateTicketRequest {
  email: string;
  message: string;
}

export interface CreateTicketResult {
  ticketId?: string;
  status: "queued";
}

interface RawResponse {
  ticket_id?: string;
  status?: string;
}

export async function createTicket(req: CreateTicketRequest): Promise<CreateTicketResult> {
  const res = await fetch("/api/support/ticket", {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ email: req.email, message: req.message }),
    cache: "no-store",
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    let message = res.statusText || `HTTP ${res.status}`;
    try {
      const parsed = JSON.parse(text) as { error?: { message?: string } };
      if (parsed?.error?.message) message = parsed.error.message;
    } catch {
      /* keep raw status */
    }
    throw new Error(message);
  }
  const raw = (await res.json().catch(() => ({}))) as RawResponse;
  return { ticketId: raw.ticket_id, status: "queued" };
}

export function useCreateTicket() {
  return useMutation<CreateTicketResult, Error, CreateTicketRequest>({
    mutationFn: createTicket,
  });
}
