export interface ToolDefinition<TArgs = unknown> {
  id: string;
  description: string;
  execute(args: TArgs): Promise<unknown>;
  risk: "read" | "write" | "execute";
}

export class ToolRegistry {
  private readonly tools = new Map<string, ToolDefinition>();

  register(tool: ToolDefinition): void {
    if (!/^[a-z0-9][a-z0-9._-]*$/.test(tool.id)) {
      throw new Error("Invalid tool id");
    }
    if (this.tools.has(tool.id)) {
      throw new Error(`Tool already registered: ${tool.id}`);
    }
    this.tools.set(tool.id, tool);
  }

  get(id: string): ToolDefinition | undefined {
    return this.tools.get(id);
  }
}
