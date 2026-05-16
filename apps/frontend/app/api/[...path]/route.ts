import { NextRequest } from "next/server";

const SHIM_URL = process.env.AGENTSPAY_SHIM_URL ?? "http://localhost:8080";

// Allow only path segments that look like the shim's actual route surface.
// Reject anything that would let URL normalization escape the /api/ prefix
// or sneak metadata characters into the upstream request line.
const SAFE_SEGMENT = /^[A-Za-z0-9._-]+$/;

function isSafeSegment(segment: string): boolean {
  if (segment.length === 0 || segment.length > 64) return false;
  if (segment === "." || segment === "..") return false;
  return SAFE_SEGMENT.test(segment);
}

async function proxy(
  req: NextRequest,
  ctx: { params: Promise<{ path: string[] }> },
) {
  const { path } = await ctx.params;

  if (path.length === 0 || path.length > 8 || !path.every(isSafeSegment)) {
    return new Response(
      JSON.stringify({ error: "path_rejected", reason: "invalid_segment" }),
      { status: 400, headers: { "content-type": "application/json" } },
    );
  }

  const targetUrl = `${SHIM_URL}/api/${path.join("/")}${req.nextUrl.search}`;

  const headers = new Headers(req.headers);
  headers.delete("host");
  headers.delete("content-length");
  // Strip any incoming X-Forwarded-For — letting the client choose the
  // shim's rate-limit key would defeat the per-IP throttling. The shim
  // trusts the X-Forwarded-For added by the proxy in front of it (Caddy
  // in prod), so anything coming in here should not be passed through.
  headers.delete("x-forwarded-for");
  headers.delete("forwarded");

  const init: RequestInit = {
    method: req.method,
    headers,
    redirect: "manual",
  };
  if (req.method !== "GET" && req.method !== "HEAD") {
    init.body = await req.arrayBuffer();
  }

  const upstream = await fetch(targetUrl, init);
  const respHeaders = new Headers(upstream.headers);
  ["connection", "transfer-encoding", "content-encoding"].forEach((h) =>
    respHeaders.delete(h),
  );
  return new Response(upstream.body, {
    status: upstream.status,
    headers: respHeaders,
  });
}

export const GET = proxy;
export const POST = proxy;
export const PUT = proxy;
export const DELETE = proxy;
export const PATCH = proxy;
