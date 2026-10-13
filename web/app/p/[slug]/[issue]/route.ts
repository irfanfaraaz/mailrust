import { EngineError, engineResponse } from "@/lib/engine";
import { archiveHeaders } from "@/lib/public-routes";

/** The engine builds the whole page (no raw HTML anywhere); this only serves it under the strict CSP. */
export async function GET(_: Request, { params }: { params: Promise<{ slug: string; issue: string }> }) {
  const { slug, issue } = await params;
  try {
    const res = await engineResponse(`/public/archive/${encodeURIComponent(slug)}/${encodeURIComponent(issue)}`, { auth: false });
    return new Response(await res.text(), { headers: archiveHeaders() });
  } catch (e) {
    if (e instanceof EngineError && e.status === 404) return new Response("Not found", { status: 404, headers: { "content-type": "text/plain" } });
    throw e;
  }
}
