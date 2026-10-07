import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";

export interface LlamaCppConfig {
  executable: string;
  modelPath: string;
  contextTokens: number;
  temperature: number;
}

export interface ProcessRunner {
  run(input: string, signal?: AbortSignal): Promise<string>;
}

/**
 * llama.cpp adapter contract. The process runner is injected so the UI never
 * receives arbitrary command execution capabilities.
 */
export class LlamaCppRuntime implements AgentRuntime {
  constructor(
    private readonly config: LlamaCppConfig,
    private readonly runner: ProcessRunner,
  ) {}

  async chat(request: ChatRequest): Promise<ChatResponse> {
    const output = await this.runner.run(request.message, request.signal);
    return {
      text: output,
      modelId: this.config.modelPath,
      runtime: "local",
    };
  }
}
