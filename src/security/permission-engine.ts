export type Permission = "read_project" | "write_project" | "run_tool" | "network";
export interface PermissionContext { approved: readonly Permission[]; }
export function createPermissionContext(approved: readonly Permission[] = []): PermissionContext {
  return { approved: [...new Set(approved)] };
}
export function requirePermission(context: PermissionContext, permission: Permission): void {
  if (!context.approved.includes(permission)) throw new Error(`Permission denied: ${permission}`);
}
