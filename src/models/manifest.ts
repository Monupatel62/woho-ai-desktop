import type { ModelDescriptor } from "../core/contracts";

export interface TrustedModel extends ModelDescriptor {
  filename: string;
  sizeBytes: number;
  sha256: string;
  downloadUrl: string;
  source: string;
  license: string;
}

export const BUILTIN_MODELS: readonly TrustedModel[] = [
  {
    id: "qwen3-0.6b-q4_0",
    name: "Qwen3 0.6B Q4_0",
    format: "gguf",
    contextTokens: 8192,
    filename: "Qwen3-0.6B-Q4_0.gguf",
    sizeBytes: 428970080,
    sha256: "da2572f16c06133561ce56accaa822216f2391ef4d37fba427801cd6736417d4",
    downloadUrl:
      "https://huggingface.co/ggml-org/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q4_0.gguf",
    source: "https://huggingface.co/ggml-org/Qwen3-0.6B-GGUF",
    license: "apache-2.0",
  },
];

export function getTrustedModel(modelId: string): TrustedModel {
  const model = BUILTIN_MODELS.find((candidate) => candidate.id === modelId);
  if (!model) {
    throw new Error("Unknown model");
  }
  return model;
}

export function validateModelDescriptor(model: ModelDescriptor): void {
  if (!/^[a-z0-9][a-z0-9._-]{0,127}$/.test(model.id)) {
    throw new Error("Invalid model id");
  }
  if (!Number.isSafeInteger(model.contextTokens) || model.contextTokens < 256) {
    throw new Error("Invalid model context");
  }
}
