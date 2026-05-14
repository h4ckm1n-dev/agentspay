export type ToolName =
  | "agentspay_balance"
  | "agentspay_pay_url"
  | "agentspay_set_budget"
  | "agentspay_audit_log"
  | "agentspay_topup_info";

export interface SessionResponse {
  session_id: string;
  expires_in_secs: number;
}

export interface CallResponse {
  session_id: string;
  tool: ToolName;
  result: { content: Array<{ type: "text"; text: string }> };
  latency_ms: number;
}

export interface ShimError {
  code: string;
  message: string;
  request_id: string;
}

async function http<T>(path: string, init: RequestInit): Promise<T> {
  const r = await fetch(path, {
    ...init,
    headers: { "content-type": "application/json", ...(init.headers || {}) },
  });
  if (!r.ok) {
    const body = (await r.json().catch(() => null)) as ShimError | null;
    throw new Error(body?.message ?? `${r.status} ${r.statusText}`);
  }
  return (await r.json()) as T;
}

let cachedSession: { id: string; loadedAt: number } | null = null;
const SESSION_REFRESH_MS = 25 * 60 * 1000;

export async function getSession(): Promise<string> {
  if (
    cachedSession &&
    Date.now() - cachedSession.loadedAt < SESSION_REFRESH_MS
  ) {
    return cachedSession.id;
  }
  const r = await http<SessionResponse>("/api/sandbox/session", {
    method: "POST",
  });
  cachedSession = { id: r.session_id, loadedAt: Date.now() };
  return r.session_id;
}

export async function callTool(
  tool: ToolName,
  args: object = {},
): Promise<CallResponse> {
  const session_id = await getSession();
  try {
    return await http<CallResponse>("/api/sandbox/call", {
      method: "POST",
      body: JSON.stringify({ session_id, tool, args }),
    });
  } catch {
    cachedSession = null;
    const retrySession = await getSession();
    return http<CallResponse>("/api/sandbox/call", {
      method: "POST",
      body: JSON.stringify({ session_id: retrySession, tool, args }),
    });
  }
}
