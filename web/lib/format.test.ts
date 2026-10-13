import { describe, expect, it } from "vitest";
import { closedMessage, isoDate } from "./format";

describe("isoDate", () => {
  it("turns a date-time input value into ISO", () => {
    expect(isoDate("2026-10-01T09:30Z")).toBe("2026-10-01T09:30:00.000Z");
  });

  it("returns undefined for garbage instead of throwing", () => {
    expect(isoDate("garbage")).toBeUndefined();
  });
});

describe("closedMessage", () => {
  it("turns a known reason into the next step", () => {
    expect(closedMessage("signups are turned off").text).toBe("Closed: signups are turned off. Tick Take signups below and save.");
  });

  it("names the unverified domain and asks for a Domains link", () => {
    expect(closedMessage("news.example.com is not a verified domain of this account")).toEqual({
      text: "Closed: news.example.com is not verified, so confirm emails cannot come from it. Verify it, or pick another domain below.",
      domains: true,
    });
  });

  it("passes an unknown reason through", () => {
    expect(closedMessage("something new")).toEqual({ text: "Closed: something new." });
  });
});
