import { Channel, invoke } from "@tauri-apps/api/core";
import type { AppError } from "@/types/generated/AppError";
import type { ErrorCode } from "@/types/generated/ErrorCode";

/** Error raised for every failed IPC call, carrying the backend's stable error code. */
export class IpcError extends Error {
  readonly code: ErrorCode;

  constructor(code: ErrorCode, message: string) {
    super(message);
    this.name = "IpcError";
    this.code = code;
  }

  static from(raw: unknown): IpcError {
    if (raw instanceof IpcError) return raw;
    if (isAppError(raw)) return new IpcError(raw.code, raw.message);
    const message = raw instanceof Error ? raw.message : String(raw);
    return new IpcError("INTERNAL", message);
  }
}

function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as AppError).code === "string"
  );
}

/**
 * Port between the frontend and the Rust backend. Services depend on this interface, never on
 * `@tauri-apps/api` directly, so they can be tested with an in-memory fake.
 */
export interface Transport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  /** Creates a value that can be passed as a command argument and receives streamed messages. */
  channel<T>(onMessage: (message: T) => void): unknown;
}

/** Real transport backed by Tauri's IPC. */
export class TauriTransport implements Transport {
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
      return await invoke<T>(command, args);
    } catch (raw) {
      throw IpcError.from(raw);
    }
  }

  channel<T>(onMessage: (message: T) => void): unknown {
    const channel = new Channel<T>();
    channel.onmessage = onMessage;
    return channel;
  }
}
