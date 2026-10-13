"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError } from "@/lib/engine";

export type TagState = { error?: string };

async function run(account: string, f: () => Promise<unknown>): Promise<TagState> {
  try {
    await f();
  } catch (e) {
    if (e instanceof EngineError && [400, 409].includes(e.status)) return { error: e.message };
    throw e;
  }
  revalidatePath(`/a/${account}/tags`);
  return {};
}

export async function createTag(account: string, _: TagState, form: FormData) {
  return run(account, () => engine("/tags", { method: "POST", account, body: { name: String(form.get("name") ?? "") } }));
}

export async function renameTag(account: string, id: string, _: TagState, form: FormData) {
  return run(account, () => engine(`/tags/${id}`, { method: "PATCH", account, body: { name: String(form.get("name") ?? "") } }));
}

export async function deleteTag(account: string, id: string) {
  await engine(`/tags/${id}`, { method: "DELETE", account });
  revalidatePath(`/a/${account}/tags`);
}
