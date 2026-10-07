import { describe, expect, it } from "vitest";
import { createDesktopAgentBridge } from "../src/runtime/desktop-agent";
import { validateRelativePath } from "../src/workspace/project";

describe("desktop agent boundary", () => {
  it("exposes a Tauri-backed AgentRuntime shape", () => {
    const bridge = createDesktopAgentBridge();
    expect(typeof bridge.chat).toBe("function");
  });

  it("keeps workspace validation intact", () => {
    expect(() => validateRelativePath("../secret")).toThrow();
    expect(() => validateRelativePath("src/main.ts")).not.toThrow();
  });
});
