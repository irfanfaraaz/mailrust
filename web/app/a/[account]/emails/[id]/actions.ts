"use server";
import { revalidatePath } from "next/cache";
import { engine } from "@/lib/engine";

export async function cancelEmail(account: string, id: string) {
  await engine(`/emails/${id}/cancel`, { method: "POST", account });
  revalidatePath(`/a/${account}/emails/${id}`);
}
