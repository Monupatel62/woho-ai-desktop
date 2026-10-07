import type { ModelDescriptor } from "../core/contracts";

export const BUILTIN_MODELS: readonly ModelDescriptor[] = [
  {
    id: "woho-model-placeholder",
    name: "WoHo Model (configured)",
    format: "gguf",
    contextTokens: 8192,
  },
];

export function validateModelDescriptor(model: ModelDescriptor): void {
  if (!/^[a-z0-9][a-z0-9._-]*$/.test(model.id)) {
    throw new Error("Invalid model id");
  }
  if (!Number.isSafeInteger(model.contextTokens) || model.contextTokens < 256) {
    throw new Error("Invalid model context");
  }
}
