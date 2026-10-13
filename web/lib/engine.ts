import "server-only";
import { cookies, headers } from "next/headers";
import { notFound, redirect } from "next/navigation";
import type { components } from "./engine-types";

export type Schemas = components["schemas"];
export const SESSION_COOKIE = "mr_session";

export class EngineError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
  }
}

type Opts = {
  method?: string;
  body?: unknown;
  /** Account id; sent as X-Mailrust-Account. */
  account?: string;
  /** false for routes that need no session (login, verify, accept). */
  auth?: boolean;
  /** Where to come back to after logging in again. */
  nextPath?: string;
};

function loginUrl(nextPath?: string) {
  return nextPath ? `/login?next=${encodeURIComponent(nextPath)}` : "/login";
}

async function clientIp(): Promise<string | null> {
  const h = await headers();
  return h.get("x-forwarded-for")?.split(",")[0]?.trim() || h.get("x-real-ip");
}

/** The only place that calls the Rust engine. */
export async function engine<T>(path: string, opts: Opts = {}): Promise<T> {
  const base = process.env.ENGINE_URL ?? "http://localhost:8080";
  const auth = opts.auth !== false;
  const h: Record<string, string> = { accept: "application/json" };
  if (opts.body !== undefined) h["content-type"] = "application/json";
  if (process.env.WEB_SECRET) {
    h["x-mailrust-web"] = process.env.WEB_SECRET;
    const ip = await clientIp();
    if (ip) h["x-mailrust-client-ip"] = ip;
  }
  if (auth) {
    const token = (await cookies()).get(SESSION_COOKIE)?.value;
    if (!token) redirect(loginUrl(opts.nextPath));
    h.authorization = `Bearer ${token}`;
  }
  if (opts.account) h["x-mailrust-account"] = opts.account;

  let res: Response;
  try {
    res = await fetch(base + path, {
      method: opts.method ?? "GET",
      headers: h,
      body: opts.body === undefined ? undefined : JSON.stringify(opts.body),
      cache: "no-store",
    });
  } catch {
    throw new EngineError(0, "engine_unreachable", "The Mailrust engine is not reachable.");
  }
  if (res.status === 401 && auth) redirect(loginUrl(opts.nextPath));
  if (res.status === 404 && opts.account) notFound();
  if (!res.ok) {
    const b = await res.json().catch(() => null);
    throw new EngineError(res.status, b?.error?.code ?? "error", b?.error?.message ?? res.statusText);
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}
