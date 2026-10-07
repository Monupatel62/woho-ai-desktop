import type { DatasetExample, WoHoDataset } from "./schema";

export interface DatasetManifest {
  schemaVersion: number;
  exampleCount: number;
  trainCount: number;
  validationCount: number;
  sourceCounts: Record<string, number>;
  sha256: string;
}

export interface DatasetSplit {
  train: DatasetExample[];
  validation: DatasetExample[];
}

const canonical = (examples: DatasetExample[]) =>
  examples.map((item) => JSON.stringify(item)).join("\n") + "\n";

async function sha256Hex(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function validateUniqueIds(examples: DatasetExample[]): void {
  const ids = new Set<string>();
  for (const example of examples) {
    if (ids.has(example.id)) throw new Error(`Duplicate dataset example ID: ${example.id}`);
    ids.add(example.id);
  }
}

export function normalizeDataset(dataset: WoHoDataset): WoHoDataset {
  validateUniqueIds(dataset.examples);
  const examples = [...dataset.examples].sort((a, b) => a.id.localeCompare(b.id));
  return { schemaVersion: dataset.schemaVersion, examples };
}

export function splitDataset(dataset: WoHoDataset, validationRatio = 0.1): DatasetSplit {
  if (!Number.isFinite(validationRatio) || validationRatio < 0 || validationRatio >= 1) {
    throw new Error("validationRatio must be >= 0 and < 1");
  }
  const normalized = normalizeDataset(dataset);
  const validationCount = Math.floor(normalized.examples.length * validationRatio);
  const splitAt = normalized.examples.length - validationCount;
  return {
    train: normalized.examples.slice(0, splitAt),
    validation: normalized.examples.slice(splitAt),
  };
}

export async function createDatasetManifest(dataset: WoHoDataset, validationRatio = 0.1): Promise<DatasetManifest> {
  const normalized = normalizeDataset(dataset);
  const split = splitDataset(normalized, validationRatio);
  const sourceCounts: Record<string, number> = {};
  for (const example of normalized.examples) {
    sourceCounts[example.source] = (sourceCounts[example.source] ?? 0) + 1;
  }
  return {
    schemaVersion: normalized.schemaVersion,
    exampleCount: normalized.examples.length,
    trainCount: split.train.length,
    validationCount: split.validation.length,
    sourceCounts,
    sha256: await sha256Hex(canonical(normalized.examples)),
  };
}
