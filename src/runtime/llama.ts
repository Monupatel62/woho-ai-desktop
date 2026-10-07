import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";
import { chatWithDesktopAgent } from "./desktop-agent";

export interface LlamaCppConfig {
  modelId: string;
}

export class LlamaCppRuntime implements AgentRuntime {
  constructor(private readonly config: LlamaCppConfig) {}

  chat(request: ChatRequest): Promise<ChatResponse> {
    if (request.runtime !== "local") {
      throw new Error("LlamaCppRuntime requires local runtime requests");
    }
    return chatWithDesktopAgent({
      conversationId: request.conversationId,
      message: request.message,
      modelId: this.config.modelId,
      runtime: "local",
    });
  }
}
