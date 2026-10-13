"use server";
import { redirect } from "next/navigation";
import { engine, EngineError, type Schemas } from "@/lib/engine";
import { setSession } from "@/lib/session";
import type { FormState } from "../login/actions";

export async function acceptInvite(_: FormState, form: FormData): Promise<FormState> {
  let s: Schemas["SessionResponse"];
  try {
    s = await engine<Schemas["SessionResponse"]>("/invites/accept", {
      method: "POST",
      body: { token: String(form.get("token") ?? "") },
      auth: false,
    });
  } catch (e) {
    if (e instanceof EngineError && e.status === 401) return { error: "This invite is invalid, used or expired. Ask for a new one." };
    throw e;
  }
  await setSession(s.session_token, s.expires_at);
  redirect("/");
}
