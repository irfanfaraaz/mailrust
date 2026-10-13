import { NextResponse, type NextRequest } from "next/server";
import { engine, EngineError } from "@/lib/engine";
import { unsubscribeIntent } from "@/lib/public-routes";

/**
 * proxy.ts rewrites plain POSTs to /u/<token> here: RFC 8058 one-click unsubscribes (urlencoded or
 * multipart), and the page's own form when JavaScript is off, which goes back to the page with ?done=.
 */
export async function POST(req: NextRequest, { params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  const form = await req.formData().catch(() => null);
  const intent = form && unsubscribeIntent(form);
  if (!intent) return new NextResponse("Expected List-Unsubscribe=One-Click", { status: 400 });
  const back = (done: string) => NextResponse.redirect(new URL(`/u/${encodeURIComponent(token)}?done=${done}`, req.url), 303);
  if (intent === "resubscribe") {
    try {
      await engine("/public/resubscribe", { method: "POST", auth: false, body: { token } });
      return back("resubscribed");
    } catch (e) {
      if (e instanceof EngineError && e.status === 409) return back("resubscribe_failed");
      throw e;
    }
  }
  await engine("/public/unsubscribe", { method: "POST", auth: false, body: { token } });
  return intent === "one-click" ? new NextResponse(null, { status: 200 }) : back("unsubscribed");
}
