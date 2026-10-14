import type { Transport } from "@/services/ipc";

type Handler = (args: Record<string, unknown>) => unknown;

/** In-memory `Transport` for tests: register a handler per command and inspect calls. */
export class FakeTransport implements Transport {
  readonly calls: { command: string; args: Record<string, unknown> }[] = [];
  private readonly handlers = new Map<string, Handler>();
  private readonly channels: ((message: unknown) => void)[] = [];

  on(command: string, handler: Handler): this {
    this.handlers.set(command, handler);
    return this;
  }

  async invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
    this.calls.push({ command, args });
    const handler = this.handlers.get(command);
    if (!handler) throw new Error(`no fake handler for ${command}`);
    return (await handler(args)) as T;
  }

  channel<T>(onMessage: (message: T) => void): unknown {
    this.channels.push(onMessage as (message: unknown) => void);
    return { channelIndex: this.channels.length - 1 };
  }

  /** Delivers a message to the most recently created channel. */
  emit(message: unknown): void {
    const last = this.channels.at(-1);
    if (!last) throw new Error("no channel created");
    last(message);
  }

  lastCall(command: string): Record<string, unknown> | undefined {
    return this.calls.filter((c) => c.command === command).at(-1)?.args;
  }
}
