"use server";
import { engine, EngineError, type Schemas } from "@/lib/engine";

export type ConfirmState = { name?: string; expired?: boolean };

export async function confirmSubscription(token: string, _: ConfirmState): Promise<ConfirmState> {
  try {
    const r = await engine<Schemas["ConfirmView"]>("/public/confirm", { method: "POST", auth: false, body: { token } });
    return { name: r.newsletter_name };
  } catch (e) {
    if (e instanceof EngineError && e.status === 401) return { expired: true };
    throw e;
  }
}
