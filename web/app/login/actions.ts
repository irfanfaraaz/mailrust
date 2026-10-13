"use server";
import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { engine, EngineError, type Schemas } from "@/lib/engine";
import { safeNext } from "@/lib/safe-next";
import { clearSession, setSession } from "@/lib/session";

export type FormState = { error?: string };

/** Carries `next` from the login form to the confirm page, since the email link cannot. */
const NEXT_COOKIE = "mr_next";

export async function requestLink(_: FormState, form: FormData): Promise<FormState> {
  const email = String(form.get("email") ?? "");
  try {
    await engine("/auth/login", { method: "POST", body: { email }, auth: false });
  } catch (e) {
    if (e instanceof EngineError && e.status === 400) return { error: "That does not look like an email address." };
    throw e;
  }
  const next = safeNext(String(form.get("next") ?? ""));
  const jar = await cookies();
  if (next !== "/") jar.set(NEXT_COOKIE, next, { maxAge: 900, httpOnly: true, sameSite: "lax", path: "/" });
  else jar.delete(NEXT_COOKIE);
  redirect("/login/check-email");
}

export async function confirmLogin(_: FormState, form: FormData): Promise<FormState> {
  let s: Schemas["SessionResponse"];
  try {
    s = await engine<Schemas["SessionResponse"]>("/auth/verify", {
      method: "POST",
      body: { token: String(form.get("token") ?? "") },
      auth: false,
    });
  } catch (e) {
    if (e instanceof EngineError && e.status === 401) return { error: "This link is invalid or has expired. Ask for a new one." };
    throw e;
  }
  await setSession(s.session_token, s.expires_at);
  const jar = await cookies();
  const next = String(form.get("next") ?? "") || jar.get(NEXT_COOKIE)?.value;
  jar.delete(NEXT_COOKIE);
  redirect(safeNext(next));
}

export async function logout() {
  await engine("/auth/logout", { method: "POST" }).catch(() => undefined);
  await clearSession();
  redirect("/login");
}

export async function logoutAll() {
  await engine("/auth/logout-all", { method: "POST" }).catch(() => undefined);
  await clearSession();
  redirect("/login");
}
