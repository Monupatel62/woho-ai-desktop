export interface GgufArtifact {
  modelId: string;
  fileName: string;
  sha256: string;
  sizeBytes: number;
  quantization: "Q4_K_M" | "Q5_K_M" | "Q8_0" | "other";
}

export function validateGgufArtifact(artifact: GgufArtifact): void {
  if (!artifact.fileName.toLowerCase().endsWith(".gguf")) {
    throw new Error("Model artifact must use GGUF format");
  }
  if (!/^[a-f0-9]{64}$/i.test(artifact.sha256)) {
    throw new Error("GGUF artifact requires SHA-256 verification");
  }
  if (!Number.isSafeInteger(artifact.sizeBytes) || artifact.sizeBytes <= 0) {
    throw new Error("Invalid GGUF artifact size");
  }
}
