import { describe, expect, it } from "vitest";
import { namesMissingText, pct, wavePlanText } from "./report";

describe("report wording", () => {
  it("formats rates", () => {
    expect(pct(0)).toBe("0%");
    expect(pct(0.25)).toBe("25%");
    expect(pct(0.0312)).toBe("3.1%");
    expect(pct(1)).toBe("100%");
  });
  it("describes the wave plan", () => {
    expect(wavePlanText([])).toBe("No readers match this audience.");
    expect(wavePlanText([42])).toBe("One wave: all 42 readers at once.");
    expect(wavePlanText([100, 200, 400, 500])).toBe("4 waves, 10 minutes apart: 100, 200, 400, 500. Most engaged readers go first.");
  });
  it("warns about missing first names only when the issue uses them", () => {
    expect(namesMissingText(312, 1200, "there")).toBe("312 of 1,200 have no first name; they will see “there”.");
    expect(namesMissingText(312, 1200, null)).toBeNull();
    expect(namesMissingText(0, 1200, "there")).toBeNull();
  });
});
