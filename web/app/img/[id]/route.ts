import { EngineError, engineResponse } from "@/lib/engine";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Image bytes for emails, the editor and the archive. Always /img/<id>, whatever store the engine uses. */
export async function GET(_: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  if (!UUID.test(id)) return new Response("Not found", { status: 404 });
  try {
    const res = await engineResponse(`/public/img/${id}`, { auth: false });
    return new Response(res.body, {
      headers: {
        "content-type": res.headers.get("content-type") ?? "application/octet-stream",
        "x-content-type-options": "nosniff",
        "cache-control": "public, max-age=31536000, immutable",
      },
    });
  } catch (e) {
    if (e instanceof EngineError && e.status === 404) return new Response("Not found", { status: 404 });
    throw e;
  }
}
