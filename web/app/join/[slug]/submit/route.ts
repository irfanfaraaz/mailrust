import { NextResponse, type NextRequest } from "next/server";
import { engine, EngineError } from "@/lib/engine";
import { DRAFT_COOKIE } from "@/lib/join-draft";

/** Takes the join form, from our page or from a form embedded on any site (proxy.ts rewrites POST /join/<slug> here). */
export async function POST(req: NextRequest, { params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const form = await req.formData().catch(() => new FormData());
  const page = `/join/${encodeURIComponent(slug)}`;
  const go = (path: string) => NextResponse.redirect(new URL(path, req.url), 303);
  const email = String(form.get("email") ?? "");
  const firstName = String(form.get("first_name") ?? "");
  try {
    await engine("/public/join", {
      method: "POST",
      auth: false,
      body: { slug, email, first_name: firstName || null, website: String(form.get("website") ?? "") },
    });
  } catch (e) {
    if (!(e instanceof EngineError)) throw e;
    if (e.status === 400) {
      // Keep what they typed for the retry: a short-lived cookie scoped to this form, never the URL.
      const res = go(`${page}?error=email`);
      res.cookies.set(DRAFT_COOKIE, JSON.stringify({ email, first_name: firstName }), { path: page, maxAge: 300, httpOnly: true, sameSite: "lax" });
      return res;
    }
    if (e.status === 409) return go(page);
    if (e.status === 404) return new NextResponse("Not found", { status: 404 });
    // Rate limited: the same page as success, so the answer says nothing about the address.
    if (e.status !== 429) throw e;
  }
  return go(`${page}/check-email`);
}
