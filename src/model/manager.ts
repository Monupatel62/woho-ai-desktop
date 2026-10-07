export interface ManagedModel {
  id: string;
  name: string;
  format: "gguf" | "remote";
  contextTokens: number;
  filename: string;
  sizeBytes: number;
  source: string;
  license: string;
  installed: boolean;
  verified: boolean;
}

export interface TauriInvoker {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

export class ModelManager {
  constructor(private readonly tauri: TauriInvoker) {}

  list(): Promise<ManagedModel[]> {
    return this.tauri.invoke<ManagedModel[]>("model_list");
  }

  install(modelId: string): Promise<ManagedModel> {
    return this.tauri.invoke<ManagedModel>("model_install", { modelId });
  }

  verify(modelId: string): Promise<ManagedModel> {
    return this.tauri.invoke<ManagedModel>("model_verify", { modelId });
  }

  remove(modelId: string): Promise<void> {
    return this.tauri.invoke<void>("model_remove", { modelId });
  }

  async ensureVerified(modelId: string): Promise<ManagedModel> {
    const model = await this.verify(modelId);
    if (!model.installed || !model.verified) {
      throw new Error(`Model is not installed or verified: ${modelId}`);
    }
    return model;
  }
}
