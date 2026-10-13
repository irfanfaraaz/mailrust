import { NextResponse, type NextRequest } from "next/server";
import { publicPostTarget } from "@/lib/public-routes";
import { refreshSessionCookie } from "@/lib/session-cookie";

export function proxy(req: NextRequest) {
  // Server actions carry a Next-Action header and stay on the page.
  const target = publicPostTarget(req.method, req.nextUrl.pathname, req.headers.has("next-action"));
  if (target) return NextResponse.rewrite(new URL(target, req.url));
  return refreshSessionCookie(req, NextResponse.next());
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
