import manifest from "./manifest.json";
import type { ModelDescriptor } from "../core/contracts";

export interface TrustedModel extends ModelDescriptor {
  filename: string;
  sizeBytes: number;
  sha256: string;
  downloadUrl: string;
  source: string;
  license: string;
}

interface ManifestShape {
  version: number;
  models: TrustedModel[];
}

export const MODEL_MANIFEST = manifest as ManifestShape;
export const BUILTIN_MODELS: readonly TrustedModel[] = MODEL_MANIFEST.models;

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
