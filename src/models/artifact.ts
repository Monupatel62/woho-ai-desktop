import type { GgufArtifact } from "./gguf";
import { validateGgufArtifact } from "./gguf";

export interface ModelArtifactManifest {
  version: 1;
  artifact: GgufArtifact;
  source: "trained" | "converted" | "imported";
  baseModel?: string;
  datasetSha256?: string;
}

export function validateModelArtifactManifest(manifest: ModelArtifactManifest): void {
  if (manifest.version !== 1) throw new Error("Unsupported model artifact manifest version");
  validateGgufArtifact(manifest.artifact);
  if (manifest.artifact.modelId.length > 128) throw new Error("Model ID is too long");
  if (manifest.baseModel && manifest.baseModel.length > 256) throw new Error("Base model is too long");
  if (manifest.datasetSha256 && !/^[a-f0-9]{64}$/i.test(manifest.datasetSha256)) {
    throw new Error("Invalid dataset SHA-256");
  }
}

export function createModelArtifactManifest(
  artifact: GgufArtifact,
  source: ModelArtifactManifest["source"],
  provenance?: Pick<ModelArtifactManifest, "baseModel" | "datasetSha256">,
): ModelArtifactManifest {
  const manifest: ModelArtifactManifest = { version: 1, artifact: structuredClone(artifact), source, ...provenance };
  validateModelArtifactManifest(manifest);
  return manifest;
}
