import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EngineError as EngineErrorT } from "./engine";

const cookieStore = new Map<string, string>();
const headerStore = new Map<string, string>();
vi.mock("server-only", () => ({}));
vi.mock("next/headers", () => ({
  cookies: async () => ({ get: (k: string) => (cookieStore.has(k) ? { value: cookieStore.get(k) } : undefined) }),
  headers: async () => ({ get: (k: string) => headerStore.get(k) ?? null }),
}));
vi.mock("next/navigation", () => ({
  redirect: (url: string) => { throw new Error(`REDIRECT ${url}`); },
  notFound: () => { throw new Error("NOT_FOUND"); },
}));

const { engine, EngineError } = await import("./engine");

function respond(status: number, body?: unknown) {
  const fetchMock = vi.fn(async () => new Response(body === undefined ? null : JSON.stringify(body), { status }));
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

beforeEach(() => {
  cookieStore.clear();
  headerStore.clear();
  process.env.ENGINE_URL = "http://engine.test";
  process.env.WEB_SECRET = "s".repeat(32);
});

describe("engine", () => {
  it("sends the session, account, secret and client ip", async () => {
    cookieStore.set("mr_session", "mr_sess_abc");
    headerStore.set("x-forwarded-for", "9.9.9.9, 10.0.0.1");
    const f = respond(200, { ok: true });
    await engine("/emails", { account: "acc-1" });
    const [url, init] = f.mock.calls[0] as unknown as [string, RequestInit];
    const h = init.headers as Record<string, string>;
    expect(url).toBe("http://engine.test/emails");
    expect(h.authorization).toBe("Bearer mr_sess_abc");
    expect(h["x-mailrust-account"]).toBe("acc-1");
    expect(h["x-mailrust-web"]).toBe("s".repeat(32));
    expect(h["x-mailrust-client-ip"]).toBe("9.9.9.9");
  });

  it("sends people with no session to login, keeping where they were", async () => {
    respond(200, {});
    await expect(engine("/me", { nextPath: "/a/1/emails" })).rejects.toThrow("REDIRECT /login?next=%2Fa%2F1%2Femails");
  });

  it("sends people with an expired session to login", async () => {
    cookieStore.set("mr_session", "mr_sess_old");
    respond(401, { error: { code: "unauthorized", message: "x" } });
    await expect(engine("/me")).rejects.toThrow("REDIRECT /login");
  });

  it("turns 404 inside an account into the not-found page", async () => {
    cookieStore.set("mr_session", "mr_sess_abc");
    respond(404, { error: { code: "not_found", message: "not found" } });
    await expect(engine("/emails/x", { account: "acc-1" })).rejects.toThrow("NOT_FOUND");
  });

  it("raises engine errors with their code and message", async () => {
    respond(409, { error: { code: "last_owner", message: "an account needs at least one owner" } });
    const err = (await engine("/team/members/1", { auth: false, method: "DELETE" }).catch((e) => e)) as EngineErrorT;
    expect(err).toBeInstanceOf(EngineError);
    expect(err.code).toBe("last_owner");
    expect(err.status).toBe(409);
  });

  it("reports an unreachable engine", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => { throw new TypeError("fetch failed"); }));
    const err = (await engine("/health", { auth: false }).catch((e) => e)) as EngineErrorT;
    expect(err.code).toBe("engine_unreachable");
  });

  it("returns undefined for empty answers", async () => {
    respond(204);
    await expect(engine("/auth/logout", { auth: false, method: "POST" })).resolves.toBeUndefined();
  });
});
