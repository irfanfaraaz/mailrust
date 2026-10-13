"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError } from "@/lib/engine";
import { changedFrom } from "@/lib/newsletter-from";

export type SaveState = { error?: string };

export async function saveNewsletter(account: string, _: SaveState, form: FormData): Promise<SaveState> {
  try {
    await engine("/newsletter", {
      method: "PATCH",
      account,
      body: {
        public_slug: String(form.get("public_slug") ?? ""),
        newsletter_name: String(form.get("newsletter_name") ?? ""),
        // Absent: the engine keeps the saved address.
        newsletter_from: changedFrom(form),
        signup_enabled: form.get("signup_enabled") === "on",
        postal_address: String(form.get("postal_address") ?? ""),
        signup_ask_name: form.get("signup_ask_name") === "on",
      },
    });
  } catch (e) {
    if (e instanceof EngineError && [400, 409, 422].includes(e.status)) return { error: e.message };
    throw e;
  }
  revalidatePath(`/a/${account}/signup-form`);
  return {};
}
