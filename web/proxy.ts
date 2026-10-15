import { NextResponse, type NextRequest } from "next/server";
import { publicPostTarget } from "@/lib/public-routes";
import { refreshSessionCookie } from "@/lib/session-cookie";

export function proxy(req: NextRequest) {
  // Showcase deploy (no engine behind it): only the landing page exists.
  if (process.env.MAILRUST_LANDING === "only" && req.nextUrl.pathname !== "/") return new NextResponse("Not found", { status: 404 });
  // Server actions carry a Next-Action header and stay on the page.
  const target = publicPostTarget(req.method, req.nextUrl.pathname, req.headers.has("next-action"));
  if (target) return NextResponse.rewrite(new URL(target, req.url));
  return refreshSessionCookie(req, NextResponse.next());
}

export const config = {
  // /img/ is public and cacheable: it must never carry a refreshed session cookie.
  matcher: ["/((?!_next/static|_next/image|favicon.ico|img/).*)"],
};
