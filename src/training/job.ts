import type { QloraConfig } from "./config";
import { validateQloraConfig } from "./qlora";

export interface TrainingJob {
  id: string;
  config: QloraConfig;
  status: "queued" | "running" | "completed" | "failed";
  createdAt: string;
}

export function createTrainingJob(id: string, config: QloraConfig): TrainingJob {
  if (!/^[a-z0-9][a-z0-9._-]{0,127}$/.test(id)) throw new Error("Invalid training job ID");
  validateQloraConfig(config);
  return { id, config: structuredClone(config), status: "queued", createdAt: new Date().toISOString() };
}
