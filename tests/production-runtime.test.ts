import { describe, expect, it } from "vitest";
import { BUILTIN_MODELS, getTrustedModel } from "../src/models/manifest";
import { getBuiltinModel } from "../src/models/model-manager";

describe("production runtime contracts", () => {
  it("keeps the trusted packaged model contract stable", () => {
    expect(BUILTIN_MODELS).toHaveLength(1);
    const model = getTrustedModel("qwen3-0.6b-q4_0");
    expect(model.filename).toBe("qwen3-0.6b-q4_0.gguf");
    expect(model.sizeBytes).toBe(428970080);
    expect(model.sha256).toHaveLength(64);
    expect(model.downloadUrl).toMatch(/^https:\/\/huggingface\.co\//);
    expect(getBuiltinModel(model.id)).toMatchObject({
      installed: false,
      verified: false,
    });
  });
});
