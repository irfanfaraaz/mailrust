import { NextResponse, type NextRequest } from "next/server";
import { refreshSessionCookie } from "@/lib/session-cookie";

export function proxy(req: NextRequest) {
  return refreshSessionCookie(req, NextResponse.next());
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
