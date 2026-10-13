import { NextResponse, type NextRequest } from "next/server";
import { engine, EngineError } from "@/lib/engine";

/** Takes the join form, from our page or from a form embedded on any site (proxy.ts rewrites POST /join/<slug> here). */
export async function POST(req: NextRequest, { params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const form = await req.formData().catch(() => new FormData());
  const page = `/join/${encodeURIComponent(slug)}`;
  const go = (path: string) => NextResponse.redirect(new URL(path, req.url), 303);
  try {
    await engine("/public/join", {
      method: "POST",
      auth: false,
      body: {
        slug,
        email: String(form.get("email") ?? ""),
        first_name: String(form.get("first_name") ?? "") || null,
        website: String(form.get("website") ?? ""),
      },
    });
  } catch (e) {
    if (!(e instanceof EngineError)) throw e;
    if (e.status === 400) return go(`${page}?error=email`);
    if (e.status === 409) return go(page);
    if (e.status === 404) return new NextResponse("Not found", { status: 404 });
    // Rate limited: the same page as success, so the answer says nothing about the address.
    if (e.status !== 429) throw e;
  }
  return go(`${page}/check-email`);
}
