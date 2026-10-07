import type { BenchmarkSummary } from "./benchmark";

export interface EvaluationReport {
  modelId: string;
  benchmark: string;
  generatedAt: string;
  summary: BenchmarkSummary;
}

export function createEvaluationReport(modelId: string, benchmark: string, summary: BenchmarkSummary): EvaluationReport {
  if (!/^[a-z0-9][a-z0-9._-]{0,127}$/.test(modelId)) throw new Error("Invalid model ID");
  if (!benchmark || benchmark.length > 128) throw new Error("Invalid benchmark name");
  return { modelId, benchmark, generatedAt: new Date().toISOString(), summary: structuredClone(summary) };
}
