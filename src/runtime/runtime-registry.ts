import type { AgentRuntime, RuntimeKind } from "../core/contracts";

export class RuntimeRegistry {
  private readonly runtimes = new Map<RuntimeKind, AgentRuntime>();
  register(kind: RuntimeKind, runtime: AgentRuntime): void {
    if (this.runtimes.has(kind)) throw new Error(`Runtime already registered: ${kind}`);
    this.runtimes.set(kind, runtime);
  }
  get(kind: RuntimeKind): AgentRuntime {
    const runtime = this.runtimes.get(kind);
    if (!runtime) throw new Error(`Runtime not registered: ${kind}`);
    return runtime;
  }
  has(kind: RuntimeKind): boolean { return this.runtimes.has(kind); }
}
