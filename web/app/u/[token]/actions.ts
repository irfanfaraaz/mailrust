"use server";
import { engine, EngineError } from "@/lib/engine";
import { RESUBSCRIBE_REFUSED, type UnsubStep } from "@/lib/public-routes";

export type UnsubState = UnsubStep;

export async function unsubscribeAction(token: string, _: UnsubState, form: FormData): Promise<UnsubState> {
  if (form.get("do") === "resubscribe") {
    try {
      await engine("/public/resubscribe", { method: "POST", auth: false, body: { token } });
      return { step: "resubscribed" };
    } catch (e) {
      if (e instanceof EngineError && e.status === 409) {
        return { step: "unsubscribed", error: RESUBSCRIBE_REFUSED };
      }
      throw e;
    }
  }
  await engine("/public/unsubscribe", { method: "POST", auth: false, body: { token } });
  return { step: "unsubscribed" };
}
