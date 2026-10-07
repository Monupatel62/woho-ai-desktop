export interface ProjectWorkspace {
  root: string;
  read(relativePath: string): Promise<string>;
  write(relativePath: string, content: string): Promise<void>;
  remove(relativePath: string): Promise<void>;
}

export function validateRelativePath(path: string): void {
  if (!path || path.includes("\0")) {
    throw new Error("Workspace path contains an invalid character");
  }

  const normalized = path.replaceAll("\\", "/");

  if (
    normalized.startsWith("/") ||
    /^[a-zA-Z]:\//.test(normalized) ||
    normalized.startsWith("//") ||
    normalized.split("/").includes("..")
  ) {
    throw new Error("Workspace path must remain inside project root");
  }
}
