export interface BenchmarkCase {
  id: string;
  prompt: string;
  expectedSignals: string[];
}

export interface BenchmarkResult {
  caseId: string;
  passed: boolean;
  score: number;
}

export function scoreResponse(response: string, expectedSignals: string[]): number {
  if (expectedSignals.length === 0) return 1;
  const normalized = response.toLowerCase();
  const hits = expectedSignals.filter((signal) =>
    normalized.includes(signal.toLowerCase()),
  ).length;
  return hits / expectedSignals.length;
}
