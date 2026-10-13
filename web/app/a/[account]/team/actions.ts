"use server";
import { redirect } from "next/navigation";
import { revalidatePath } from "next/cache";
import { engine, EngineError } from "@/lib/engine";

export type TeamState = { error?: string };

async function run(account: string, f: () => Promise<unknown>): Promise<TeamState> {
  try {
    await f();
    revalidatePath(`/a/${account}/team`);
    return {};
  } catch (e) {
    if (e instanceof EngineError && [400, 403, 409, 422].includes(e.status)) return { error: e.message };
    throw e;
  }
}

export async function invite(account: string, _: TeamState, form: FormData) {
  return run(account, () => engine("/team/invites", { method: "POST", account, body: { email: String(form.get("email") ?? ""), role: String(form.get("role") ?? "member") } }));
}
export async function cancelInvite(account: string, id: string, _: TeamState) {
  return run(account, () => engine(`/team/invites/${id}`, { method: "DELETE", account }));
}
export async function setRole(account: string, userId: string, _: TeamState, form: FormData) {
  return run(account, () => engine(`/team/members/${userId}`, { method: "PATCH", account, body: { role: String(form.get("role")) } }));
}
export async function removeMember(account: string, userId: string, _: TeamState) {
  return run(account, () => engine(`/team/members/${userId}`, { method: "DELETE", account }));
}
export async function leave(account: string, userId: string, _: TeamState): Promise<TeamState> {
  const r = await run(account, () => engine(`/team/members/${userId}`, { method: "DELETE", account }));
  if (r.error) return r;
  redirect("/");
}
