import { describe, expect, it } from "vitest";
import { plain } from "./plain";

describe("plain", () => {
  it("turns null-prototype objects (Tiptap attrs) into plain objects a server action can send", () => {
    const attrs = Object.assign(Object.create(null), { label: "Go", href: "https://a.com" });
    const doc = { type: "doc", content: [{ type: "button", attrs }] };

    const out = plain(doc);

    expect(Object.getPrototypeOf(out.content[0].attrs)).toBe(Object.prototype);
    expect(out).toEqual({ type: "doc", content: [{ type: "button", attrs: { label: "Go", href: "https://a.com" } }] });
  });
});
