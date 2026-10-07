export type Permission = "read_project" | "write_project" | "run_tool" | "network";

export interface PermissionContext {
  sessionId: string;
  approved: ReadonlySet<Permission>;
}

export function requirePermission(
  context: PermissionContext,
  permission: Permission,
): void {
  if (!context.approved.has(permission)) {
    throw new Error(`Permission denied: ${permission}`);
  }
}
