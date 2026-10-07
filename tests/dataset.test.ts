import { describe, expect, it } from "vitest";
import { datasetToJsonl, loadDataset, writeDataset } from "../src/dataset/loader";

const valid = {
  schemaVersion: 1,
  examples: [{
    id: "welcome-001",
    instruction: "Greet the user",
    input: "Hello",
    output: "Hello! How can I help?",
    source: "curated",
    license: "apache-2.0",
    qualityScore: 0.95,
  }],
};

describe("WoHo dataset architecture", () => {
  it("round-trips the versioned dataset contract", () => {
    const dataset = loadDataset(JSON.stringify(valid));
    expect(dataset.schemaVersion).toBe(1);
    expect(writeDataset(dataset)).toBe(JSON.stringify(valid) + "\n");
  });

  it("exports deterministic JSONL records", () => {
    const dataset = loadDataset(JSON.stringify(valid));
    expect(datasetToJsonl(dataset)).toBe(JSON.stringify(valid.examples[0]) + "\n");
  });

  it("rejects path-like IDs, oversized content and invalid quality", () => {
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], id: "../escape" }],
    }))).toThrow();
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], instruction: "x".repeat(64 * 1024 + 1) }],
    }))).toThrow();
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], qualityScore: 2 }],
    }))).toThrow();
  });

  it("rejects unsupported versions and unknown sources", () => {
    expect(() => loadDataset(JSON.stringify({ ...valid, schemaVersion: 2 }))).toThrow();
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], source: "external" }],
    }))).toThrow();
  });
});
