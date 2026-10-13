import { describe, expect, it } from "vitest";
import { formatFields, parseFields } from "./fields";

describe("fields text", () => {
  it("round-trips name=value lines", () => {
    const f = { plan: "pro", note: "a=b" };
    expect(parseFields(formatFields(f))).toEqual({ value: f });
  });
  it("skips blank lines, trims, splits on the first = and takes CRLF", () => {
    expect(parseFields("\r\n plan = pro \r\n\r\nurl=https://x.test/?a=1\r\n")).toEqual({ value: { plan: "pro", url: "https://x.test/?a=1" } });
  });
  it("names the line that has no name", () => {
    expect(parseFields("plan=pro\njust text")).toEqual({ error: "Line 2: write each field as name=value." });
    expect(parseFields("=x")).toEqual({ error: "Line 1: write each field as name=value." });
  });
});
