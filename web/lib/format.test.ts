import { describe, expect, it } from "vitest";
import { isoDate } from "./format";

describe("isoDate", () => {
  it("turns a date-time input value into ISO", () => {
    expect(isoDate("2026-10-01T09:30Z")).toBe("2026-10-01T09:30:00.000Z");
  });

  it("returns undefined for garbage instead of throwing", () => {
    expect(isoDate("garbage")).toBeUndefined();
  });
});
