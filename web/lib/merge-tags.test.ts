import { describe, expect, it } from "vitest";
import { mergeTagLabel, mergeTagProblem } from "./merge-tags";

describe("mergeTagProblem", () => {
  it("needs a fallback for every key but email", () => {
    expect(mergeTagProblem("first_name", "there")).toBeNull();
    expect(mergeTagProblem("first_name", "  ")).toMatch(/fallback/);
    expect(mergeTagProblem("last_name", "")).toMatch(/fallback/);
    expect(mergeTagProblem("email", "")).toBeNull();
    expect(mergeTagProblem("plan", "free")).toBeNull();
  });
  it("checks the key and the fallback length like the engine", () => {
    expect(mergeTagProblem("Plan", "x")).toMatch(/field name/);
    expect(mergeTagProblem("a-b", "x")).toMatch(/field name/);
    expect(mergeTagProblem("x".repeat(41), "x")).toMatch(/field name/);
    expect(mergeTagProblem("first_name", "é".repeat(100))).toBeNull();
    expect(mergeTagProblem("first_name", "x".repeat(101))).toMatch(/100/);
  });
});

describe("mergeTagLabel", () => {
  it("shows the key and the fallback", () => {
    expect(mergeTagLabel("first_name", " there ")).toBe("{{first_name | there}}");
    expect(mergeTagLabel("email", "")).toBe("{{email}}");
  });
});
