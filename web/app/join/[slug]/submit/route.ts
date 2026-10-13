import { NextResponse, type NextRequest } from "next/server";
import { engine, EngineError } from "@/lib/engine";
import { HONEYPOT } from "@/lib/embed";
import { DRAFT_COOKIE, draftCookieOptions, joinErrorField } from "@/lib/join-draft";
import { seeOther } from "@/lib/public-routes";

/** Takes the join form, from our page or from a form embedded on any site (proxy.ts rewrites POST /join/<slug> here). */
export async function POST(req: NextRequest, { params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const form = await req.formData().catch(() => new FormData());
  const page = `/join/${encodeURIComponent(slug)}`;
  const email = String(form.get("email") ?? "");
  const firstName = String(form.get("first_name") ?? "");
  try {
    await engine("/public/join", {
      method: "POST",
      auth: false,
      body: { slug, email, first_name: firstName || null, website: String(form.get(HONEYPOT) ?? "") },
    });
  } catch (e) {
    if (!(e instanceof EngineError)) throw e;
    if (e.status === 400) {
      // Keep what they typed for the retry: a short-lived cookie scoped to this form, never the URL.
      const res = seeOther(`${page}?error=${joinErrorField(e.message)}`);
      res.cookies.set(DRAFT_COOKIE, JSON.stringify({ email, first_name: firstName }), draftCookieOptions(page));
      return res;
    }
    if (e.status === 409) return seeOther(page);
    if (e.status === 404) return new NextResponse("Not found", { status: 404 });
    // Rate limited: the same page as success, so the answer says nothing about the address.
    if (e.status !== 429) throw e;
  }
  return seeOther(`${page}/check-email`);
}
