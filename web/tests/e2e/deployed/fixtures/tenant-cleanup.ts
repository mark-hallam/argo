/**
 * Per-run tenant teardown for the deployed-real E2E lane.
 *
 * Every deployed spec creates a real tenant + user_account + magic-link
 * token on the live gateway. We DELETE those rows at end of test via an
 * admin endpoint so the production DB stays clean. Tenants are all
 * prefixed `e2e-test-` so the cleanup target is unambiguous.
 *
 * The admin endpoint contract:
 *   DELETE  {gatewayUrl}/admin/e2e/tenants
 *   Header  Authorization: Bearer ${ARGO_E2E_ADMIN_TOKEN}
 *   Body    { "email": "e2e-test-...@throwaway.local" }
 *   Response 200 { "deleted": { "tenants": N, "users": N, "tokens": N } }
 *
 * The endpoint is restricted to e2e-test-prefixed emails on the
 * gateway side — there's no way to delete production data via this
 * surface. If the token is unset or the endpoint doesn't exist (older
 * deploy), cleanup silently logs and returns — the throwaway email is
 * disposable, so the row leak is bounded and operator-visible.
 */

export interface CleanupResult {
  ok: boolean;
  reason?: string;
  deleted?: { tenants: number; users: number; tokens: number };
}

export async function deleteTestTenant(
  gatewayUrl: string,
  email: string,
  adminToken: string | undefined,
  fetchImpl: typeof fetch = fetch,
): Promise<CleanupResult> {
  if (!adminToken) {
    return { ok: false, reason: "ARGO_E2E_ADMIN_TOKEN unset" };
  }
  if (!email.startsWith("e2e-test-")) {
    return { ok: false, reason: `refuse: non-e2e prefix on ${email}` };
  }
  try {
    const res = await fetchImpl(`${gatewayUrl.replace(/\/+$/, "")}/admin/e2e/tenants`, {
      method: "DELETE",
      headers: {
        "content-type": "application/json",
        authorization: `Bearer ${adminToken}`,
      },
      body: JSON.stringify({ email }),
    });
    if (!res.ok) {
      return { ok: false, reason: `admin endpoint ${res.status}` };
    }
    const body = (await res.json()) as { deleted?: CleanupResult["deleted"] };
    return { ok: true, deleted: body.deleted };
  } catch (err) {
    return { ok: false, reason: err instanceof Error ? err.message : String(err) };
  }
}
