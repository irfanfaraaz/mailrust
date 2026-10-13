"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError, type Schemas } from "@/lib/engine";

export type CreateKeyState = { created?: Schemas["CreatedKey"]; error?: string };

export async function createKey(account: string, _: CreateKeyState, form: FormData): Promise<CreateKeyState> {
  try {
    const created = await engine<Schemas["CreatedKey"]>("/api-keys", {
      method: "POST",
      account,
      body: { name: String(form.get("name") ?? ""), access: String(form.get("access") ?? "full") },
    });
    revalidatePath(`/a/${account}/api-keys`);
    return { created };
  } catch (e) {
    if (e instanceof EngineError && e.status === 400) return { error: e.message };
    throw e;
  }
}

export async function deleteKey(account: string, id: string) {
  await engine(`/api-keys/${id}`, { method: "DELETE", account });
  revalidatePath(`/a/${account}/api-keys`);
}
