import { QloraConfig } from "./config";

export function validateQloraConfig(config: QloraConfig): void {
  if (!config.baseModel || !config.datasetManifest || !config.outputDir) throw new Error("Invalid QLoRA configuration");
  if (!Number.isInteger(config.rank) || config.rank < 1 || config.rank > 256) throw new Error("Invalid LoRA rank");
  if (!Number.isInteger(config.alpha) || config.alpha < 1 || config.alpha > 1024) throw new Error("Invalid LoRA alpha");
  if (!Number.isFinite(config.dropout) || config.dropout < 0 || config.dropout > 1) throw new Error("Invalid LoRA dropout");
  if (!Number.isInteger(config.epochs) || config.epochs < 1 || config.epochs > 100) throw new Error("Invalid epochs");
  if (!Number.isFinite(config.learningRate) || config.learningRate <= 0 || config.learningRate > 1) throw new Error("Invalid learning rate");
}
