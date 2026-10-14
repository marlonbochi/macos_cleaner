import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatCount,
  formatDate,
  formatDateTime,
  parentPath,
  setFormatLocale,
  shortenPath,
} from "./format";

describe("formatBytes", () => {
  it("uses base 1000 with the locale's decimal separator", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
    expect(formatBytes(1_500_000, "pt-BR")).toBe("1,5 MB");
    expect(formatBytes(5_000_000_000, "pt-BR")).toBe("5,0 GB");
    expect(formatBytes(1_500_000, "en-US")).toBe("1.5 MB");
  });

  it("follows the active format locale by default", () => {
    setFormatLocale("pt-BR");
    expect(formatBytes(1_500_000)).toBe("1,5 MB");
    setFormatLocale("en-US");
    expect(formatBytes(1_500_000)).toBe("1.5 MB");
    expect(formatCount(1234567)).toBe("1,234,567");
  });

  it("guards against invalid values", () => {
    expect(formatBytes(-1)).toBe("—");
    expect(formatBytes(Number.NaN)).toBe("—");
  });
});

describe("formatCount / dates", () => {
  it("formats counts with thousand separators", () => {
    expect(formatCount(1234567, "pt-BR")).toBe("1.234.567");
  });

  it("returns a dash for missing dates", () => {
    expect(formatDate(null)).toBe("—");
    expect(formatDateTime(undefined)).toBe("—");
  });

  it("formats epoch seconds", () => {
    expect(formatDate(0, "en-US")).toMatch(/1\/1\/70|12\/31\/69/);
    expect(formatDateTime(0, "en-US")).toMatch(/,/);
  });
});

describe("paths", () => {
  it("replaces home with ~", () => {
    expect(shortenPath("/Users/a/Movies/x.mov", "/Users/a")).toBe("~/Movies/x.mov");
    expect(shortenPath("/Users/a", "/Users/a")).toBe("~");
    expect(shortenPath("/Users/ab/x", "/Users/a")).toBe("/Users/ab/x");
    expect(shortenPath("/x", null)).toBe("/x");
  });

  it("computes the parent directory", () => {
    expect(parentPath("/Users/a/Movies")).toBe("/Users/a");
    expect(parentPath("/Users/a/Movies/")).toBe("/Users/a");
    expect(parentPath("/Users")).toBe("/");
    expect(parentPath("/")).toBe("/");
  });
});
