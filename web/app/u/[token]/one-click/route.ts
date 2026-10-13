import { NextResponse, type NextRequest } from "next/server";
import { engine } from "@/lib/engine";
import { isOneClick } from "@/lib/public-routes";

/** RFC 8058 one-click unsubscribe (proxy.ts rewrites POST /u/<token> here). urlencoded or multipart bodies. */
export async function POST(req: NextRequest, { params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  const form = await req.formData().catch(() => null);
  if (!form || !isOneClick(form)) return new NextResponse("Expected List-Unsubscribe=One-Click", { status: 400 });
  await engine("/public/unsubscribe", { method: "POST", auth: false, body: { token } });
  return new NextResponse(null, { status: 200 });
}
