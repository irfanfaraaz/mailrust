"use server";
import { redirect } from "next/navigation";
import { engine, type Schemas } from "@/lib/engine";

export async function createCampaign(account: string) {
  const c = await engine<Schemas["Campaign"]>("/campaigns", { method: "POST", account, body: {} });
  redirect(`/a/${account}/campaigns/${c.id}`);
}
