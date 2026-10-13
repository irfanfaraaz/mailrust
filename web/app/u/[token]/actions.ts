"use server";
import { engine, EngineError } from "@/lib/engine";

export type UnsubState = { step: "ask" | "unsubscribed" | "resubscribed"; error?: string };

export async function unsubscribeAction(token: string, _: UnsubState, form: FormData): Promise<UnsubState> {
  if (form.get("do") === "resubscribe") {
    try {
      await engine("/public/resubscribe", { method: "POST", auth: false, body: { token } });
      return { step: "resubscribed" };
    } catch (e) {
      if (e instanceof EngineError && e.status === 409) {
        return { step: "unsubscribed", error: "This link cannot subscribe you again. Use the newsletter's signup form." };
      }
      throw e;
    }
  }
  await engine("/public/unsubscribe", { method: "POST", auth: false, body: { token } });
  return { step: "unsubscribed" };
}
