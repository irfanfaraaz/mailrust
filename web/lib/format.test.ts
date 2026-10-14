import { describe, expect, it } from "vitest";
import { closedMessage, contactCount, isoDate, when } from "./format";

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

describe("contactCount", () => {
  it("says the total, and the page size when paged", () => {
    expect(contactCount(1, 1, false)).toBe("1 contact");
    expect(contactCount(3, 3, false)).toBe("3 contacts");
    expect(contactCount(1234, 50, true)).toBe("Showing 50 of 1,234 contacts");
  });
});

describe("when", () => {
  it("shows a timestamp in the given time zone, labelled with that zone", () => {
    expect(when("2026-10-14T08:33:00Z", "Asia/Kolkata")).toBe("14 Oct 2026, 14:03 GMT+5:30");
  });

  it("shows only the date when asked", () => {
    expect(when("2026-10-14T20:00:00Z", "Asia/Kolkata", { dateOnly: true })).toBe("15 Oct 2026");
  });
});
