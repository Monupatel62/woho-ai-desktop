export interface DatasetExample {
  id: string;
  instruction: string;
  input?: string;
  output: string;
  source: "synthetic" | "curated" | "project";
  license: string;
  qualityScore: number;
}

export function validateDatasetExample(example: DatasetExample): void {
  if (!example.id || !example.instruction || !example.output) {
    throw new Error("Dataset example is incomplete");
  }
  if (!Number.isFinite(example.qualityScore) || example.qualityScore < 0 || example.qualityScore > 1) {
    throw new Error("Dataset qualityScore must be between 0 and 1");
  }
}
