import type { AgentRuntime, ChatRequest, ChatResponse } from "../core/contracts";
import { requirePermission, type PermissionContext } from "../security/permission-engine";

export interface CodingAgentRequest extends ChatRequest { permissions: PermissionContext; }

export class CodingAgent {
  constructor(private readonly runtime: AgentRuntime) {}
  async run(request: CodingAgentRequest): Promise<ChatResponse> {
    requirePermission(request.permissions, "read_project");
    return this.runtime.chat(request);
  }
}
