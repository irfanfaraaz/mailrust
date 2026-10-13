import { describe, expect, it } from "vitest";
import { BLOCK_ITEMS, filterBlocks } from "./block-menu";

describe("filterBlocks", () => {
  it("lists every block for an empty query", () => {
    expect(filterBlocks("").map((b) => b.id)).toEqual(BLOCK_ITEMS.map((b) => b.id));
    expect(BLOCK_ITEMS).toHaveLength(11);
  });
  it("matches titles and keyword starts, ignoring case", () => {
    expect(filterBlocks("Head").map((b) => b.id)).toEqual(["h1", "h2", "h3"]);
    expect(filterBlocks("list").map((b) => b.id)).toEqual(["bullets", "numbers"]);
    expect(filterBlocks("img").map((b) => b.id)).toEqual(["image"]);
    expect(filterBlocks("name").map((b) => b.id)).toEqual(["merge"]);
    expect(filterBlocks("cta").map((b) => b.id)).toEqual(["button"]);
    expect(filterBlocks("zzz")).toEqual([]);
  });
});
