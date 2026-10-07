import { describe, expect, it } from "vitest";
import { scoreBenchmark } from "../src/evaluation/benchmark";
import { createPermissionContext, requirePermission } from "../src/security/permission-engine";
import { InMemoryStore } from "../src/memory/memory-store";
import { RuntimeRegistry } from "../src/runtime/runtime-registry";
import { validateQloraConfig } from "../src/training/qlora";
import { DEFAULT_QLORA_CONFIG } from "../src/training/config";
import { validateWorkspace } from "../src/workspace/workspace";

describe("remaining architecture foundations", () => {
  it("validates QLoRA configuration", () => {
    expect(() => validateQloraConfig(DEFAULT_QLORA_CONFIG)).not.toThrow();
    expect(() => validateQloraConfig({ ...DEFAULT_QLORA_CONFIG, rank: 0 })).toThrow();
  });
  it("scores benchmarks deterministically", () => {
    const result = scoreBenchmark([{ id: "a", prompt: "1+1", expected: "2" }], new Map([["a", " 2 "]]));
    expect(result.average).toBe(1);
  });
  it("enforces explicit permissions", () => {
    expect(() => requirePermission(createPermissionContext(), "write_project")).toThrow("Permission denied");
  });
  it("isolates memory records", async () => {
    const store = new InMemoryStore();
    await store.put({ id: "m1", content: "hello", createdAt: 1 });
    const record = await store.get("m1");
    expect(record).toEqual({ id: "m1", content: "hello", createdAt: 1 });
  });
  it("registers runtimes exactly once", () => {
    const registry = new RuntimeRegistry();
    const runtime = { chat: async () => ({ text: "ok", modelId: "x", runtime: "local" as const }) };
    registry.register("local", runtime);
    expect(registry.has("local")).toBe(true);
    expect(() => registry.register("local", runtime)).toThrow();
  });
  it("validates workspace roots", () => {
    expect(validateWorkspace("C:/work").allowed).toBe(true);
    expect(() => validateWorkspace("")).toThrow();
  });
});
