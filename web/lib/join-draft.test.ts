import { afterEach, describe, expect, it, vi } from "vitest";
import { draftCookieOptions, joinErrorField, readDraft } from "./join-draft";

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

describe("joinErrorField", () => {
  it("blames the email only when the engine says the email is bad", () => {
    expect(joinErrorField("nope is not a valid email address")).toBe("email");
    expect(joinErrorField("names are at most 100 characters")).toBe("name");
    expect(joinErrorField("names cannot hold line breaks or control characters")).toBe("name");
  });
});

describe("draftCookieOptions", () => {
  afterEach(() => vi.unstubAllEnvs());
  it("is short-lived, scoped to the form and http-only", () => {
    expect(draftCookieOptions("/join/weekly")).toMatchObject({ path: "/join/weekly", maxAge: 300, httpOnly: true, sameSite: "lax" });
  });
  it("is secure in production, like the session cookie", () => {
    vi.stubEnv("NODE_ENV", "production");
    expect(draftCookieOptions("/join/weekly").secure).toBe(true);
    vi.stubEnv("COOKIE_SECURE", "false");
    expect(draftCookieOptions("/join/weekly").secure).toBe(false);
  });
  it("is not secure in development", () => {
    vi.stubEnv("NODE_ENV", "development");
    expect(draftCookieOptions("/join/weekly").secure).toBe(false);
  });
});
