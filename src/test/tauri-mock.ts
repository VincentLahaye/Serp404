import { vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

const mockedInvoke = vi.mocked(invoke);

export function mockInvoke(responses: Record<string, unknown>) {
  mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
    if (cmd in responses) {
      const response = responses[cmd];
      return typeof response === "function" ? response(args) : response;
    }
    throw new Error(`Unmocked invoke: ${cmd}`);
  });
  return mockedInvoke;
}

export function resetInvokeMock() {
  mockedInvoke.mockReset();
}

export { mockedInvoke };
