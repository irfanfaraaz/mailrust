"use server";
import { revalidatePath } from "next/cache";
import { engine, EngineError } from "@/lib/engine";

export type AddSuppressionState = { error?: string };

export async function addSuppression(account: string, _: AddSuppressionState, form: FormData): Promise<AddSuppressionState> {
  try {
    await engine("/suppressions", { method: "POST", account, body: { address: String(form.get("address") ?? "") } });
    revalidatePath(`/a/${account}/suppressions`);
    return {};
  } catch (e) {
    if (e instanceof EngineError && e.status === 400) return { error: e.message };
    throw e;
  }
}

export async function removeSuppression(account: string, address: string) {
  await engine(`/suppressions/${encodeURIComponent(address)}`, { method: "DELETE", account });
  revalidatePath(`/a/${account}/suppressions`);
}
