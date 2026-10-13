import { describe, expect, it } from "vitest";
import { changedFrom } from "./newsletter-from";

const form = (entries: Record<string, string>) => {
  const f = new FormData();
  for (const [k, v] of Object.entries(entries)) f.set(k, v);
  return f;
};

describe("changedFrom", () => {
  it("leaves the saved address alone when the user did not change it", () => {
    expect(changedFrom(form({ from_local: "hello", from_domain: "old.com", from_current: "hello@old.com" }))).toBeUndefined();
  });
  it("leaves it alone when the form had no from inputs (no verified domain left)", () => {
    expect(changedFrom(form({ from_current: "hello@old.com" }))).toBeUndefined();
  });
  it("sends a new address, or an empty one to clear it", () => {
    expect(changedFrom(form({ from_local: "hi", from_domain: "new.com", from_current: "hello@old.com" }))).toBe("hi@new.com");
    expect(changedFrom(form({ from_local: "", from_domain: "new.com", from_current: "hello@old.com" }))).toBe("");
    expect(changedFrom(form({ from_local: "hi", from_domain: "new.com", from_current: "" }))).toBe("hi@new.com");
  });
});
