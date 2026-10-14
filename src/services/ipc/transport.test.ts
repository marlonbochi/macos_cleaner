import { afterEach, describe, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { IpcError, TauriTransport } from "./transport";

describe("TauriTransport", () => {
  afterEach(() => clearMocks());

  it("returns the command result", async () => {
    mockIPC((cmd, args) => (cmd === "echo" ? args : null));
    const result = await new TauriTransport().invoke("echo", { value: 1 });
    expect(result).toEqual({ value: 1 });
  });

  it("converts backend AppError rejections into IpcError with the same code", async () => {
    mockIPC(() => {
      throw { code: "SCAN_IN_PROGRESS", message: "busy" };
    });
    const error = await new TauriTransport().invoke("start_scan").catch((e: unknown) => e);
    expect(error).toBeInstanceOf(IpcError);
    expect(error).toMatchObject({ code: "SCAN_IN_PROGRESS", message: "busy" });
  });
});

describe("IpcError.from", () => {
  it("wraps unknown errors as INTERNAL", () => {
    expect(IpcError.from(new Error("boom"))).toMatchObject({ code: "INTERNAL", message: "boom" });
    expect(IpcError.from("text")).toMatchObject({ code: "INTERNAL", message: "text" });
  });

  it("keeps existing IpcError instances", () => {
    const original = new IpcError("NOT_FOUND", "x");
    expect(IpcError.from(original)).toBe(original);
  });
});
