import type { ModelArtifactManifest } from "../models/artifact";
import { validateModelArtifactManifest } from "../models/artifact";

export interface ModelRuntimeArtifact {
  modelId: string;
  fileName: string;
  sha256: string;
  sizeBytes: number;
}

export function resolveRuntimeArtifact(manifest: ModelArtifactManifest): ModelRuntimeArtifact {
  validateModelArtifactManifest(manifest);
  return {
    modelId: manifest.artifact.modelId,
    fileName: manifest.artifact.fileName,
    sha256: manifest.artifact.sha256.toLowerCase(),
    sizeBytes: manifest.artifact.sizeBytes,
  };
}
