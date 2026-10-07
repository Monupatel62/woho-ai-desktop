export interface QloraConfig {
  baseModel: string;
  datasetManifest: string;
  outputDir: string;
  rank: number;
  alpha: number;
  dropout: number;
  epochs: number;
  learningRate: number;
  quantization: "4bit" | "8bit";
}

export const DEFAULT_QLORA_CONFIG: QloraConfig = {
  baseModel: "open-model-placeholder",
  datasetManifest: "dataset/manifest.jsonl",
  outputDir: "artifacts/training",
  rank: 16,
  alpha: 32,
  dropout: 0.05,
  epochs: 2,
  learningRate: 0.0002,
  quantization: "4bit",
};
