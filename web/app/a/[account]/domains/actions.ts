"use server";
import { redirect } from "next/navigation";
import { engine, EngineError, type Schemas } from "@/lib/engine";

export type AddDomainState = { error?: string };

export async function addDomain(account: string, _: AddDomainState, form: FormData): Promise<AddDomainState> {
  let created: Schemas["Domain"];
  try {
    created = await engine<Schemas["Domain"]>("/domains", {
      method: "POST",
      account,
      body: {
        name: String(form.get("name") ?? "").trim(),
        tracking_subdomain: String(form.get("tracking_subdomain") ?? "").trim() || null,
        open_tracking: form.has("open_tracking"),
        click_tracking: form.has("click_tracking"),
      },
    });
  } catch (e) {
    if (e instanceof EngineError && [400, 409, 422].includes(e.status)) return { error: e.message };
    throw e;
  }
  redirect(`/a/${account}/domains/${created.id}`);
}
