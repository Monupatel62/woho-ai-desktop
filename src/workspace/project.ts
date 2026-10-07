export interface ProjectWorkspace {
  root: string;
  read(relativePath: string): Promise<string>;
  write(relativePath: string, content: string): Promise<void>;
  remove(relativePath: string): Promise<void>;
}

export function validateRelativePath(path: string): void {
  if (!path || path.includes("\0") || path.startsWith("/") || /^[a-zA-Z]:/.test(path)) {
    throw new Error("Workspace path must be relative");
  }
  const normalized = path.replaceAll("\\", "/");
  if (normalized.split("/").includes("..")) {
    throw new Error("Workspace path escapes project root");
  }
}
