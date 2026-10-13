"use server";
import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { engine, EngineError } from "@/lib/engine";
import { parseFields } from "@/lib/fields";

export type ContactState = { error?: string };

const text = (form: FormData, k: string) => String(form.get(k) ?? "");
const tagList = (s: string) => s.split(",").map((t) => t.trim()).filter(Boolean);

async function run(f: () => Promise<unknown>, path: string): Promise<ContactState> {
  try {
    await f();
  } catch (e) {
    if (e instanceof EngineError && [400, 409].includes(e.status)) return { error: e.message };
    throw e;
  }
  revalidatePath(path);
  return {};
}

export async function addContact(account: string, _: ContactState, form: FormData) {
  return run(
    () => engine("/contacts", {
      method: "POST",
      account,
      body: { email: text(form, "email"), first_name: text(form, "first_name"), last_name: text(form, "last_name"), tags: tagList(text(form, "tags")) },
    }),
    `/a/${account}/contacts`,
  );
}

export async function updateContact(account: string, id: string, _: ContactState, form: FormData) {
  const fields = parseFields(text(form, "fields"));
  if ("error" in fields) return { error: fields.error };
  return run(
    () => engine(`/contacts/${id}`, {
      method: "PATCH",
      account,
      body: { first_name: text(form, "first_name"), last_name: text(form, "last_name"), tags: tagList(text(form, "tags")), fields: fields.value },
    }),
    `/a/${account}/contacts/${id}`,
  );
}

export async function unsubscribeContact(account: string, id: string) {
  await engine(`/contacts/${id}`, { method: "PATCH", account, body: { status: "unsubscribed" } });
  revalidatePath(`/a/${account}/contacts/${id}`);
}

export async function deleteContact(account: string, id: string) {
  await engine(`/contacts/${id}`, { method: "DELETE", account });
  revalidatePath(`/a/${account}/contacts`);
  redirect(`/a/${account}/contacts`);
}
