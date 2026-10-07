import { invoke } from "@tauri-apps/api/core";
import type { ModelDescriptor } from "../core/contracts";
import { getTrustedModel, validateModelDescriptor } from "./manifest";

export interface ModelSource {
  url: string;
  sha256: string;
}

export interface ManagedModel extends ModelDescriptor {
  filename: string;
  installed: boolean;
  verified: boolean;
  sizeBytes: number;
  source: string;
  license: string;
}

export interface ModelManager {
  install(model: ModelDescriptor, source: ModelSource): Promise<void>;
  remove(modelId: string): Promise<void>;
}

export function validateModelSource(source: ModelSource): void {
  const url = new URL(source.url);
  if (url.protocol !== "https:" || url.hostname !== "huggingface.co") {
    throw new Error("Model downloads require HTTPS from the trusted Hugging Face host");
  }
  if (!/^[a-f0-9]{64}$/i.test(source.sha256)) {
    throw new Error("Model source must include a SHA-256 digest");
  }
}

export function validateInstall(model: ModelDescriptor, source: ModelSource): void {
  validateModelDescriptor(model);
  validateModelSource(source);
}

export function getBuiltinModel(modelId: string): ManagedModel {
  const model = getTrustedModel(modelId);
  return {
    ...model,
    installed: false,
    verified: false,
  };
}

export async function listModels(): Promise<ManagedModel[]> {
  return invoke<ManagedModel[]>("model_list");
}

export async function installModel(modelId: string): Promise<ManagedModel> {
  getTrustedModel(modelId);
  return invoke<ManagedModel>("model_install", { modelId });
}

export async function removeModel(modelId: string): Promise<void> {
  getTrustedModel(modelId);
  await invoke("model_remove", { modelId });
}

export async function verifyModel(modelId: string): Promise<ManagedModel> {
  getTrustedModel(modelId);
  return invoke<ManagedModel>("model_verify", { modelId });
}

export async function ensureVerifiedModel(modelId: string): Promise<ManagedModel> {
  const model = await verifyModel(modelId);
  if (!model.installed || !model.verified) {
    throw new Error("Model is not installed or verified");
  }
  return model;
}
