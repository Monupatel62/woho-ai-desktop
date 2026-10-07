import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";

export interface CloudProvider {
  chat(request: ChatRequest): Promise<ChatResponse>;
}

export class CloudRuntime implements AgentRuntime {
  constructor(private readonly provider: CloudProvider) {}

  chat(request: ChatRequest): Promise<ChatResponse> {
    if (request.runtime !== "cloud") {
      throw new Error("CloudRuntime requires cloud runtime requests");
    }
    return this.provider.chat(request);
  }
}
