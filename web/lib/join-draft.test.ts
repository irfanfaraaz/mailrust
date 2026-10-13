import { describe, expect, it } from "vitest";
import { readDraft } from "./join-draft";

describe("readDraft", () => {
  it("reads the typed values back", () => {
    expect(readDraft(JSON.stringify({ email: "a@b", first_name: "Ana" }))).toEqual({ email: "a@b", first_name: "Ana" });
  });

  it("returns empty values for a missing, broken or tampered cookie", () => {
    expect(readDraft(undefined)).toEqual({ email: "", first_name: "" });
    expect(readDraft("{not json")).toEqual({ email: "", first_name: "" });
    expect(readDraft(JSON.stringify({ email: 1, first_name: ["x"] }))).toEqual({ email: "", first_name: "" });
  });
});
