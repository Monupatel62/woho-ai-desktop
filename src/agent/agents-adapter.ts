import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";

/**
 * Integration boundary for @woho/agents.
 *
 * The desktop app owns the UI/process boundary; @woho/agents remains the
 * orchestration engine. The concrete adapter is isolated here so package
 * upgrades cannot leak through the UI.
 */
export interface WohoAgentsBridge {
  run(request: ChatRequest): Promise<ChatResponse>;
}

export function createAgentsBridge(_runtime: AgentRuntime): WohoAgentsBridge {
  return {
    async run(request) {
      return _runtime.chat(request);
    },
  };
}
