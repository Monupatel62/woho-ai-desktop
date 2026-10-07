import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";
import { requirePermission, type PermissionContext } from "../security/permissions";

export interface CodingTask {
  sessionId: string;
  projectRoot: string;
  instruction: string;
}

export class CodingAgent {
  constructor(
    private readonly runtime: AgentRuntime,
    private readonly permissions: PermissionContext,
  ) {}

  async plan(task: CodingTask): Promise<ChatResponse> {
    requirePermission(this.permissions, "read_project");
    const request: ChatRequest = {
      conversationId: task.sessionId,
      message: `Plan the coding task in project ${task.projectRoot}: ${task.instruction}`,
      modelId: "woho-model-placeholder",
      runtime: "local",
    };
    return this.runtime.chat(request);
  }
}
