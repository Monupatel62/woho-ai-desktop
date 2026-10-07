import { invoke } from "@tauri-apps/api/core";
import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";

export interface DesktopAgentRequest {
  conversationId: string;
  message: string;
  modelId: string;
  runtime: "local";
}

export interface DesktopAgentBridge extends AgentRuntime {
  chat(request: ChatRequest): Promise<ChatResponse>;
}

export async function chatWithDesktopAgent(request: DesktopAgentRequest): Promise<ChatResponse> {
  if (request.runtime !== "local") throw new Error("Desktop agent bridge only supports local runtime");
  return invoke<ChatResponse>("agent_chat", {
    request: {
      conversationId: request.conversationId,
      message: request.message,
      modelId: request.modelId,
      runtime: request.runtime,
    },
  });
}

export function createDesktopAgentBridge(): DesktopAgentBridge {
  return { chat: chatWithDesktopAgent };
}
