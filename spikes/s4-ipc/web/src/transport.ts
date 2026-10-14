// T0.S4.4: the same control messages and binary frames over Tauri IPC and over WebSocket.
// The page doesn't know which core it talks to beyond picking a transport.

import type { DiffKind, Encoding, Intent, Scenario } from "./types";

export type Command =
  | { cmd: "start"; scenario: Scenario; diff: DiffKind; enc: Encoding }
  | { cmd: "next" }
  | { cmd: "dispatch"; intent: Intent }
  | { cmd: "final" }
  | { cmd: "results"; data: unknown };

export interface Transport {
  readonly name: string;
  /** Sends a command and returns the reply (`dispatch` has none: resolves at once). */
  send(cmd: Command): Promise<ArrayBuffer>;
  close(): void;
}

const utf8 = new TextDecoder();

export function text(b: ArrayBuffer): string {
  return utf8.decode(b);
}

interface TauriGlobal {
  core: { invoke(cmd: string, args: Record<string, unknown>): Promise<unknown> };
}

export function tauriAvailable(): boolean {
  return "__TAURI__" in window;
}

export function tauriTransport(): Transport {
  const tauri = (window as unknown as { __TAURI__: TauriGlobal }).__TAURI__;
  return {
    name: "tauri",
    async send(cmd) {
      // The shell answers with a raw `tauri::ipc::Response`: an ArrayBuffer, no JSON.
      const reply = await tauri.core.invoke("ipc", { msg: JSON.stringify(cmd) });
      if (reply instanceof ArrayBuffer) return reply;
      return new Uint8Array(reply as number[]).buffer;
    },
    close() {},
  };
}

export function wsTransport(url: string): Promise<Transport> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    // Replies come back in order, one per command that has one.
    const waiting: ((b: ArrayBuffer) => void)[] = [];
    ws.onmessage = (e: MessageEvent<ArrayBuffer | string>) => {
      const next = waiting.shift();
      const data = e.data;
      next?.(typeof data === "string" ? new TextEncoder().encode(data).buffer : data);
    };
    ws.onerror = () => reject(new Error(`websocket ${url} failed`));
    ws.onopen = () =>
      resolve({
        name: "ws",
        send(cmd) {
          ws.send(JSON.stringify(cmd));
          if (cmd.cmd === "dispatch") return Promise.resolve(new ArrayBuffer(0));
          return new Promise((r) => waiting.push(r));
        },
        close() {
          ws.close();
        },
      });
  });
}
