"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError } from "@/lib/engine";

export type SaveState = { error?: string };

export async function saveNewsletter(account: string, _: SaveState, form: FormData): Promise<SaveState> {
  const local = String(form.get("from_local") ?? "").trim();
  const domain = String(form.get("from_domain") ?? "");
  try {
    await engine("/newsletter", {
      method: "PATCH",
      account,
      body: {
        public_slug: String(form.get("public_slug") ?? ""),
        newsletter_name: String(form.get("newsletter_name") ?? ""),
        newsletter_from: local && domain ? `${local}@${domain}` : "",
        signup_enabled: form.get("signup_enabled") === "on",
      },
    });
  } catch (e) {
    if (e instanceof EngineError && [400, 409, 422].includes(e.status)) return { error: e.message };
    throw e;
  }
  revalidatePath(`/a/${account}/signup-form`);
  return {};
}
