import { describe, expect, it } from "vitest";
import { formatMs, latencyLevel, percentLevel, pingLevel } from "./format";

describe("formatMs", () => {
  it("shows a zero reading for unusable values", () => {
    expect(formatMs(0)).toBe("0 ms");
    expect(formatMs(-3)).toBe("0 ms");
    expect(formatMs(Number.NaN)).toBe("0 ms");
    expect(formatMs(Number.POSITIVE_INFINITY)).toBe("0 ms");
  });

  it("keeps two decimals under a millisecond", () => {
    expect(formatMs(0.456)).toBe("0.46 ms");
  });

  it("keeps one decimal under ten milliseconds", () => {
    expect(formatMs(9.94)).toBe("9.9 ms");
  });

  it("rounds larger values", () => {
    expect(formatMs(10.4)).toBe("10 ms");
    expect(formatMs(124.6)).toBe("125 ms");
  });
});

describe("latencyLevel", () => {
  it("changes level at the documented thresholds", () => {
    expect(latencyLevel(9.9)).toBe("ok");
    expect(latencyLevel(10)).toBe("warn");
    expect(latencyLevel(39.9)).toBe("warn");
    expect(latencyLevel(40)).toBe("bad");
  });
});

describe("pingLevel", () => {
  it("changes level at the documented thresholds", () => {
    expect(pingLevel(19.9)).toBe("ok");
    expect(pingLevel(20)).toBe("warn");
    expect(pingLevel(99.9)).toBe("warn");
    expect(pingLevel(100)).toBe("bad");
  });
});

describe("percentLevel", () => {
  it("is ok below 80 % and warns from 80 %", () => {
    expect(percentLevel(0)).toBe("ok");
    expect(percentLevel(79.9)).toBe("ok");
    expect(percentLevel(80)).toBe("warn");
    expect(percentLevel(89.9)).toBe("warn");
    expect(percentLevel(90)).toBe("bad");
    expect(percentLevel(100)).toBe("bad");
  });
});
