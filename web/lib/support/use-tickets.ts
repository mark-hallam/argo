"use client";

/**
 * Operator inbox hooks (Stream `claude-support-inbox-spa-page`).
 *
 * Backend (admin-only, X-Argo-Role: operator):
 *   - GET    /api/v1/support/tickets?status=open&limit=50
 *   - GET    /api/v1/support/tickets/<id>
 *   - POST   /api/v1/support/tickets/<id>/reply
 *   - PATCH  /api/v1/support/tickets/<id>
 *
 * The sibling `use-create-ticket.ts` still owns the user-facing
 * create path through the Next.js `/api/support/ticket` proxy.
 */

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPatch, apiPost } from "@/lib/api/client";

export type TicketStatus = "open" | "pending" | "resolved" | "closed";
export type TicketPriority = "low" | "normal" | "high" | "urgent";
export type ReplyAuthorKind = "user" | "operator" | "system";

export interface SupportTicket {
  id: string;
  tenantId: string | null;
  userEmail: string;
  subject: string;
  body: string;
  status: TicketStatus;
  priority: TicketPriority;
  createdAt: number;
  updatedAt: number;
  assignedTo: string | null;
}

export interface SupportReply {
  id: string;
  ticketId: string;
  authorKind: ReplyAuthorKind;
  authorId: string | null;
  body: string;
  createdAt: number;
}

export interface TicketDetail {
  ticket: SupportTicket;
  replies: SupportReply[];
}

interface TicketJson {
  id: string;
  tenant_id: string | null;
  user_email: string;
  subject: string;
  body: string;
  status: TicketStatus;
  priority: TicketPriority;
  created_at: number;
  updated_at: number;
  assigned_to: string | null;
}

interface ReplyJson {
  id: string;
  ticket_id: string;
  author_kind: ReplyAuthorKind;
  author_id: string | null;
  body: string;
  created_at: number;
}

interface ListJson {
  tickets: TicketJson[];
}

interface DetailJson {
  ticket: TicketJson;
  replies: ReplyJson[];
}

function rowToTicket(row: TicketJson): SupportTicket {
  return {
    id: row.id,
    tenantId: row.tenant_id ?? null,
    userEmail: row.user_email,
    subject: row.subject,
    body: row.body,
    status: row.status,
    priority: row.priority,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    assignedTo: row.assigned_to ?? null,
  };
}

function rowToReply(row: ReplyJson): SupportReply {
  return {
    id: row.id,
    ticketId: row.ticket_id,
    authorKind: row.author_kind,
    authorId: row.author_id ?? null,
    body: row.body,
    createdAt: row.created_at,
  };
}

const LIST_KEY = (status?: TicketStatus) =>
  status ? (["support-tickets", status] as const) : (["support-tickets", "all"] as const);
const DETAIL_KEY = (id: string) => ["support-ticket", id] as const;

export async function fetchTickets(status?: TicketStatus): Promise<SupportTicket[]> {
  const params = new URLSearchParams();
  if (status) params.set("status", status);
  params.set("limit", "100");
  const path = `/api/v1/support/tickets?${params.toString()}`;
  const raw = await apiGet<ListJson>(path);
  return (raw.tickets ?? []).map(rowToTicket);
}

export async function fetchTicket(id: string): Promise<TicketDetail> {
  const raw = await apiGet<DetailJson>(`/api/v1/support/tickets/${id}`);
  return {
    ticket: rowToTicket(raw.ticket),
    replies: (raw.replies ?? []).map(rowToReply),
  };
}

export interface PostReply {
  ticketId: string;
  body: string;
}

export interface UpdateTicket {
  ticketId: string;
  status?: TicketStatus;
  priority?: TicketPriority;
  assignedTo?: string | null;
}

export function useTickets(status?: TicketStatus) {
  return useQuery({
    queryKey: LIST_KEY(status),
    queryFn: () => fetchTickets(status),
  });
}

export function useTicket(id: string) {
  return useQuery({
    queryKey: DETAIL_KEY(id),
    queryFn: () => fetchTicket(id),
    enabled: id.length > 0,
  });
}

export function usePostReply() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ ticketId, body }: PostReply) => {
      await apiPost<{ reply_id: string }>(
        `/api/v1/support/tickets/${ticketId}/reply`,
        { body },
      );
      return { ticketId };
    },
    onSuccess: ({ ticketId }) => {
      void qc.invalidateQueries({ queryKey: DETAIL_KEY(ticketId) });
      void qc.invalidateQueries({ queryKey: ["support-tickets"] });
    },
  });
}

export function useUpdateTicket() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ ticketId, status, priority, assignedTo }: UpdateTicket) => {
      const body: Record<string, unknown> = {};
      if (status) body.status = status;
      if (priority) body.priority = priority;
      if (assignedTo !== undefined) body.assigned_to = assignedTo;
      const raw = await apiPatch<{ ticket: TicketJson }>(
        `/api/v1/support/tickets/${ticketId}`,
        body,
      );
      return rowToTicket(raw.ticket);
    },
    onSuccess: (ticket) => {
      void qc.invalidateQueries({ queryKey: DETAIL_KEY(ticket.id) });
      void qc.invalidateQueries({ queryKey: ["support-tickets"] });
    },
  });
}
