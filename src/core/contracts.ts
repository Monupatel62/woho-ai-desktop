export type RuntimeKind = "local" | "cloud";

export interface ChatRequest {
  conversationId: string;
  message: string;
  modelId: string;
  runtime: RuntimeKind;
  signal?: AbortSignal;
}

export interface ChatResponse {
  text: string;
  modelId: string;
  runtime: RuntimeKind;
}

export interface AgentRuntime {
  chat(request: ChatRequest): Promise<ChatResponse>;
}

export interface ModelDescriptor {
  id: string;
  name: string;
  format: "gguf" | "remote";
  contextTokens: number;
  sizeBytes?: number;
  sha256?: string;
}
