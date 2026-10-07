import { describe, expect, it } from "vitest";
import { DEFAULT_QLORA_CONFIG } from "../src/training/config";
import { createTrainingJob } from "../src/training/job";
import { buildTrainingCommand, validateTrainingCommand } from "../src/training/executor";
import { createModelArtifactManifest } from "../src/models/artifact";
import { resolveRuntimeArtifact } from "../src/runtime/model-artifact";

describe("training execution and model artifact contracts", () => {
  it("builds a deterministic, shell-free training argv", () => {
    const job = createTrainingJob("train-001", DEFAULT_QLORA_CONFIG);
    const command = buildTrainingCommand(job);
    expect(command.executable).toBe("python");
    expect(command.args).toContain("--base-model");
    expect(command.args).toContain(DEFAULT_QLORA_CONFIG.baseModel);
    expect(command.args).not.toContain("&&");
    expect(() => buildTrainingCommand(job, "../python")).toThrow();
  });

  it("rejects unsafe training executable names", () => {
    expect(() => validateTrainingCommand({ executable: "python.exe", args: [], cwd: "training" })).not.toThrow();
    expect(() => validateTrainingCommand({ executable: "C:\\python.exe", args: [], cwd: "training" })).toThrow();
    expect(() => validateTrainingCommand({ executable: "python", args: new Array(65).fill("x"), cwd: "training" })).toThrow();
  });

  it("binds GGUF identity and provenance to runtime", () => {
    const manifest = createModelArtifactManifest({
      modelId: "woho-test",
      fileName: "woho-test.gguf",
      sha256: "a".repeat(64),
      sizeBytes: 1024,
      quantization: "Q4_K_M",
    }, "trained", { baseModel: "qwen3", datasetSha256: "b".repeat(64) });
    expect(resolveRuntimeArtifact(manifest)).toEqual({
      modelId: "woho-test",
      fileName: "woho-test.gguf",
      sha256: "a".repeat(64),
      sizeBytes: 1024,
    });
    expect(() => createModelArtifactManifest({
      modelId: "woho-test",
      fileName: "woho-test.bin",
      sha256: "a".repeat(64),
      sizeBytes: 1024,
      quantization: "other",
    }, "converted")).toThrow();
  });
});
