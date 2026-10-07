import { describe, expect, it } from "vitest";
import { datasetToJsonl, loadDataset, writeDataset } from "../src/dataset/loader";

const valid = {
  schemaVersion: 1,
  examples: [{
    id: "welcome-001",
    messages: [
      { role: "user", content: "Hello" },
      { role: "assistant", content: "Hello! How can I help?" },
    ],
    metadata: { source: "seed", language: "en" },
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

  it("rejects path-like and oversized identifiers", () => {
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], id: "../escape" }],
    }))).toThrow();
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], messages: [
        { role: "user", content: "x".repeat(64 * 1024 + 1) },
        { role: "assistant", content: "ok" },
      ]}],
    }))).toThrow();
  });

  it("rejects unsupported schema versions and invalid roles", () => {
    expect(() => loadDataset(JSON.stringify({ ...valid, schemaVersion: 2 }))).toThrow();
    expect(() => loadDataset(JSON.stringify({
      ...valid, examples: [{ ...valid.examples[0], messages: [
        { role: "tool", content: "x" }, { role: "assistant", content: "ok" },
      ]}],
    }))).toThrow();
  });
});
