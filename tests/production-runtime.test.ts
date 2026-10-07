import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

describe("production runtime contracts", () => {
  it("runs llama.cpp as a bounded single-turn chat process", () => {
    const bridge = readFileSync(new URL("../src/agent/agent-bridge.mjs", import.meta.url), "utf8");
    expect(bridge).toContain('"-cnv"');
    expect(bridge).toContain('"-st"');
    expect(bridge).toContain('"--jinja"');
    expect(bridge).toContain('"--no-display-prompt"');
    expect(bridge).toContain("llama.cpp execution timed out");
    expect(bridge).toContain("windowsHide: true");
  });

  it("keeps the trusted model manifest aligned with the packaged runtime contract", () => {
    const manifest = JSON.parse(
      readFileSync(new URL("../src/models/manifest.json", import.meta.url), "utf8"),
    );
    expect(manifest.version).toBe(1);
    expect(manifest.models).toHaveLength(1);
    expect(manifest.models[0].filename).toBe("qwen3-0.6b-q4_0.gguf");
    expect(manifest.models[0].sizeBytes).toBe(428970080);
    expect(manifest.models[0].sha256).toHaveLength(64);
    expect(manifest.models[0].downloadUrl).toMatch(/^https:\/\/huggingface\.co\//);
  });
});
