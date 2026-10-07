import { describe, expect, it } from "vitest";
import { createDatasetManifest, normalizeDataset, splitDataset } from "../src/dataset/pipeline";
import { createTrainingJob } from "../src/training/job";
import { DEFAULT_QLORA_CONFIG } from "../src/training/config";
import { createEvaluationReport } from "../src/evaluation/report";

const example = (id: string) => ({
  id,
  instruction: "Answer",
  input: "Question",
  output: "Answer",
  source: "curated" as const,
  license: "apache-2.0",
  qualityScore: 1,
});

describe("production pipeline contracts", () => {
  it("normalizes deterministically and rejects duplicate IDs", () => {
    const dataset = { schemaVersion: 1 as const, examples: [example("b"), example("a")] };
    expect(normalizeDataset(dataset).examples.map((x) => x.id)).toEqual(["a", "b"]);
    expect(() => normalizeDataset({ schemaVersion: 1, examples: [example("a"), example("a")] })).toThrow();
  });

  it("creates deterministic train/validation manifests", async () => {
    const dataset = { schemaVersion: 1 as const, examples: ["a", "b", "c", "d", "e"].map(example) };
    const split = splitDataset(dataset, 0.2);
    expect(split.train.map((x) => x.id)).toEqual(["a", "b", "c", "d"]);
    expect(split.validation.map((x) => x.id)).toEqual(["e"]);
    const first = await createDatasetManifest(dataset, 0.2);
    const second = await createDatasetManifest(dataset, 0.2);
    expect(first).toEqual(second);
    expect(first.trainCount).toBe(4);
  });

  it("creates validated training jobs", () => {
    const job = createTrainingJob("train-qwen-001", DEFAULT_QLORA_CONFIG);
    expect(job.status).toBe("queued");
    expect(job.config).not.toBe(DEFAULT_QLORA_CONFIG);
    expect(() => createTrainingJob("../escape", DEFAULT_QLORA_CONFIG)).toThrow();
  });

  it("creates validated evaluation reports", () => {
    const report = createEvaluationReport("qwen3-0.6b-q4_0", "smoke", {
      average: 1, passed: 1, total: 1, results: [{ id: "case-1", score: 1 }],
    });
    expect(report.summary.average).toBe(1);
    expect(() => createEvaluationReport("../model", "smoke", report.summary)).toThrow();
  });
});
