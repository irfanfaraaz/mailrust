"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError, type Schemas } from "@/lib/engine";

type Result = { error?: string };
const userError = (e: unknown) => e instanceof EngineError && [400, 409, 413, 415, 422, 429].includes(e.status);

async function run(f: () => Promise<unknown>): Promise<Result> {
  try {
    await f();
    return {};
  } catch (e) {
    if (userError(e)) return { error: (e as EngineError).message };
    throw e;
  }
}

export type Draft = { subject: string; preview_text: string; blocks: unknown };

export async function saveDraft(account: string, id: string, draft: Draft): Promise<Result> {
  return run(() => engine(`/campaigns/${id}`, { method: "PATCH", account, body: draft }));
}

export async function setAudience(account: string, id: string, include: string[], exclude: string[]): Promise<Result> {
  return run(() => engine(`/campaigns/${id}`, { method: "PATCH", account, body: { include_tags: include, exclude_tags: exclude } }));
}

export async function uploadImage(account: string, form: FormData): Promise<{ id: string } | { error: string }> {
  try {
    const r = await engine<Schemas["ImageUploaded"]>("/images", { method: "POST", account, body: form });
    return { id: r.id };
  } catch (e) {
    if (userError(e)) return { error: (e as EngineError).message };
    throw e;
  }
}

export async function previewCampaign(account: string, id: string): Promise<Schemas["CampaignPreview"]> {
  return engine<Schemas["CampaignPreview"]>(`/campaigns/${id}/preview`, { method: "POST", account });
}

export async function sendTest(account: string, id: string, to: string[]): Promise<Result & { queued?: number }> {
  try {
    const r = await engine<Schemas["TestSendResult"]>(`/campaigns/${id}/test`, { method: "POST", account, body: { to } });
    return { queued: r.queued };
  } catch (e) {
    if (userError(e)) return { error: (e as EngineError).message };
    throw e;
  }
}

/** Used by the send dialog and the report (Task 16). */
export async function changeStatus(account: string, id: string, action: "send" | "pause" | "resume" | "cancel", sendAt?: string): Promise<Result> {
  const r = await run(() => engine(`/campaigns/${id}/${action}`, { method: "POST", account, body: action === "send" ? { send_at: sendAt } : undefined }));
  revalidatePath(`/a/${account}/campaigns/${id}`);
  return r;
}

export async function setArchiveHidden(account: string, id: string, hidden: boolean) {
  await engine(`/campaigns/${id}`, { method: "PATCH", account, body: { archive_hidden: hidden } });
  revalidatePath(`/a/${account}/campaigns/${id}`);
}
