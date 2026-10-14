/** A plain-JSON copy. Server actions send null-prototype objects (Tiptap attrs) as opaque "$T" references, so the server gets nothing. */
export const plain = <T>(v: T): T => JSON.parse(JSON.stringify(v));
