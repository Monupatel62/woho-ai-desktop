import type { ModelDescriptor } from "../core/contracts";
import { validateModelDescriptor } from "./manifest";

export interface ModelSource {
  url: string;
  sha256: string;
}

export interface ModelManager {
  install(model: ModelDescriptor, source: ModelSource): Promise<void>;
  remove(modelId: string): Promise<void>;
}

export function validateModelSource(source: ModelSource): void {
  const url = new URL(source.url);
  if (!["https:", "file:"].includes(url.protocol)) {
    throw new Error("Model downloads require HTTPS or an explicit local file source");
  }
  if (!/^[a-f0-9]{64}$/i.test(source.sha256)) {
    throw new Error("Model source must include a SHA-256 digest");
  }
}

export function validateInstall(model: ModelDescriptor, source: ModelSource): void {
  validateModelDescriptor(model);
  validateModelSource(source);
}
