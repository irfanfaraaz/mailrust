import { engineResponse } from "@/lib/engine";

/** Streams the engine's CSV straight through as a download. */
export async function GET(_: Request, { params }: { params: Promise<{ account: string }> }) {
  const { account } = await params;
  const res = await engineResponse("/contacts/export", { account, nextPath: `/a/${account}/contacts` });
  const day = new Date().toISOString().slice(0, 10);
  return new Response(res.body, {
    headers: {
      "content-type": "text/csv; charset=utf-8",
      "content-disposition": `attachment; filename="contacts-${day}.csv"`,
      "cache-control": "no-store",
    },
  });
}
