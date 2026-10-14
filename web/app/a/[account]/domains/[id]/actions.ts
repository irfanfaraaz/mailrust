"use server";
import { revalidatePath } from "next/cache";
import { engine } from "@/lib/engine";

export async function verifyDomain(account: string, id: string) {
  await engine(`/domains/${id}/verify`, { method: "POST", account });
  revalidatePath(`/a/${account}/domains/${id}`);
}

export async function updateTracking(account: string, id: string, form: FormData) {
  await engine(`/domains/${id}`, {
    method: "PATCH",
    account,
    body: { open_tracking: form.has("open_tracking"), click_tracking: form.has("click_tracking") },
  });
  revalidatePath(`/a/${account}/domains/${id}`);
}

export async function deleteDomain(account: string, id: string) {
  await engine(`/domains/${id}`, { method: "DELETE", account });
  revalidatePath(`/a/${account}/domains`);
  return { goto: `/a/${account}/domains` }; // the client goes there after its toast; a server redirect would drop the toast
}
