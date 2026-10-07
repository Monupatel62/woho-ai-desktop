import { describe, expect, it } from "vitest";
import {
  BUILTIN_MODELS,
  getTrustedModel,
} from "../src/models/manifest";
import {
  getBuiltinModel,
  validateInstall,
  validateModelSource,
} from "../src/models/model-manager";

describe("trusted model manager", () => {
  it("pins a real GGUF artifact to a trusted source and digest", () => {
    expect(BUILTIN_MODELS).toHaveLength(1);
    const model = getTrustedModel("qwen3-0.6b-q4_0");
    expect(model.filename).toBe("qwen3-0.6b-q4_0.gguf");
    expect(model.sizeBytes).toBe(428970080);
    expect(model.sha256).toBe(
      "da2572f16c06133561ce56accaa822216f2391ef4d37fba427801cd6736417d4",
    );
    expect(model.downloadUrl).toMatch(
      /^https:\/\/huggingface\.co\/ggml-org\//,
    );
  });

  it("rejects untrusted download sources", () => {
    expect(() =>
      validateModelSource({
        url: "http://huggingface.co/example.gguf",
        sha256: "a".repeat(64),
      }),
    ).toThrow();
    expect(() =>
      validateModelSource({
        url: "https://evil.example/model.gguf",
        sha256: "a".repeat(64),
      }),
    ).toThrow();
  });

  it("rejects malformed model metadata", () => {
    expect(() =>
      validateInstall(
        {
          id: "../escape",
          name: "bad",
          format: "gguf",
          contextTokens: 8192,
        },
        {
          url: "https://huggingface.co/model.gguf",
          sha256: "a".repeat(64),
        },
      ),
    ).toThrow();
  });

  it("exposes uninstalled trusted models deterministically", () => {
    expect(getBuiltinModel("qwen3-0.6b-q4_0")).toMatchObject({
      installed: false,
      verified: false,
      id: "qwen3-0.6b-q4_0",
    });
  });
});
