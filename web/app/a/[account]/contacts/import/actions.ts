"use server";
import { redirect } from "next/navigation";
import { engine, EngineError, type Schemas } from "@/lib/engine";

export type UploadState = { error?: string };

export async function uploadImport(account: string, _: UploadState, form: FormData): Promise<UploadState> {
  const file = form.get("file");
  if (!(file instanceof File) || file.size === 0) return { error: "Choose a CSV file." };
  const body = new FormData();
  body.set("file", file);
  body.set("consent", form.get("consent") === "on" ? "true" : "false");
  const tag = String(form.get("tag_id") ?? "");
  if (tag) body.set("tag_id", tag);
  let job: Schemas["ImportView"];
  try {
    job = await engine<Schemas["ImportView"]>("/contacts/imports", { method: "POST", account, body });
  } catch (e) {
    if (e instanceof EngineError && [400, 413, 422].includes(e.status)) return { error: e.message };
    throw e;
  }
  redirect(`/a/${account}/contacts/import?job=${job.id}`);
}
