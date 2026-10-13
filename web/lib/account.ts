import "server-only";
import { notFound } from "next/navigation";
import { engine, type Schemas } from "./engine";

export function getMe(nextPath: string) {
  return engine<Schemas["MeResponse"]>("/me", { nextPath });
}

export async function currentAccount(accountId: string, nextPath: string) {
  const me = await getMe(nextPath);
  const account = me.accounts.find((a) => a.id === accountId);
  if (!account) notFound();
  return { me, account };
}
