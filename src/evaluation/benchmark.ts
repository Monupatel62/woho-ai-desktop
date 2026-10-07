export interface BenchmarkCase { id: string; prompt: string; expected: string; }
export interface BenchmarkResult { id: string; score: number; }
export interface BenchmarkSummary { average: number; passed: number; total: number; results: BenchmarkResult[]; }

const normalize = (value: string) => value.trim().toLowerCase().replace(/\s+/g, " ");

export function scoreBenchmark(cases: readonly BenchmarkCase[], outputs: ReadonlyMap<string, string>): BenchmarkSummary {
  const results = cases.map((item) => ({ id: item.id, score: normalize(outputs.get(item.id) ?? "") === normalize(item.expected) ? 1 : 0 }));
  const total = results.length;
  const passed = results.filter((r) => r.score === 1).length;
  return { average: total ? passed / total : 0, passed, total, results };
}
