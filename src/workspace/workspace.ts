import { requirePermission, type PermissionContext } from "../security/permission-engine";

export interface Workspace { root: string; allowed: boolean; }

export function validateWorkspace(root: string): Workspace {
  if (!root || root.length > 4096 || root.includes("\0")) throw new Error("Invalid workspace root");
  return { root, allowed: true };
}

export function requireWorkspaceWrite(context: PermissionContext, workspace: Workspace): void {
  if (!workspace.allowed) throw new Error("Workspace is not allowed");
  requirePermission(context, "write_project");
}
