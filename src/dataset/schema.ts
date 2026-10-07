export const DATASET_SCHEMA_VERSION = 1 as const;

export type DatasetSource = "synthetic" | "curated" | "project";

export interface DatasetExample {
  id: string;
  instruction: string;
  input?: string;
  output: string;
  source: DatasetSource;
  license: string;
  qualityScore: number;
}

export interface WoHoDataset {
  schemaVersion: typeof DATASET_SCHEMA_VERSION;
  examples: DatasetExample[];
}

const ID = /^[a-z0-9][a-z0-9._-]{0,127}$/;

function requiredText(value: unknown, field: string, max = 64 * 1024): asserts value is string {
  if (typeof value !== "string" || value.length === 0 || value.length > max) {
    throw new Error(`Invalid ${field}`);
  }
}

export function validateDatasetExample(example: DatasetExample): void {
  if (!example || typeof example !== "object") throw new Error("Invalid dataset example");
  requiredText(example.id, "example id", 128);
  if (!ID.test(example.id)) throw new Error("Invalid example id");
  requiredText(example.instruction, "instruction");
  if (example.input !== undefined) requiredText(example.input, "input");
  requiredText(example.output, "output");
  if (!["synthetic", "curated", "project"].includes(example.source)) throw new Error("Invalid dataset source");
  requiredText(example.license, "license", 256);
  if (!Number.isFinite(example.qualityScore) || example.qualityScore < 0 || example.qualityScore > 1) {
    throw new Error("Dataset qualityScore must be between 0 and 1");
  }
}

export function validateDataset(dataset: unknown): asserts dataset is WoHoDataset {
  if (!dataset || typeof dataset !== "object") throw new Error("Dataset must be an object");
  const value = dataset as Partial<WoHoDataset>;
  if (value.schemaVersion !== DATASET_SCHEMA_VERSION) throw new Error("Unsupported dataset schema version");
  if (!Array.isArray(value.examples) || value.examples.length === 0) throw new Error("Dataset must contain examples");
  if (value.examples.length > 100_000) throw new Error("Dataset contains too many examples");
  for (const example of value.examples) validateDatasetExample(example);
}

export function parseDatasetJson(input: string): WoHoDataset {
  if (new TextEncoder().encode(input).byteLength > 16 * 1024 * 1024) throw new Error("Dataset exceeds 16 MiB");
  const parsed: unknown = JSON.parse(input);
  validateDataset(parsed);
  return parsed;
}

export function serializeDataset(dataset: WoHoDataset): string {
  validateDataset(dataset);
  return JSON.stringify(dataset) + "\n";
}
