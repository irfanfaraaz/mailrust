"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError, type Schemas } from "@/lib/engine";

export type CreateWebhookState = { created?: Schemas["WebhookView"]; error?: string };

export async function createWebhook(account: string, _: CreateWebhookState, form: FormData): Promise<CreateWebhookState> {
  try {
    const created = await engine<Schemas["WebhookView"]>("/webhooks", {
      method: "POST",
      account,
      body: { url: String(form.get("url") ?? ""), events: form.getAll("events").map(String) },
    });
    revalidatePath(`/a/${account}/webhooks`);
    return { created };
  } catch (e) {
    if (e instanceof EngineError && (e.status === 400 || e.status === 422)) return { error: e.message };
    throw e;
  }
}

export async function deleteWebhook(account: string, id: string) {
  await engine(`/webhooks/${id}`, { method: "DELETE", account });
  revalidatePath(`/a/${account}/webhooks`);
}
