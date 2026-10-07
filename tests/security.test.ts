import { describe, expect, it } from "vitest";
import { validateRelativePath } from "../src/workspace/project";
import { requirePermission } from "../src/security/permissions";

describe("desktop security boundaries", () => {
  it("rejects workspace traversal", () => {
    expect(() => validateRelativePath("../secret")).toThrow();
    expect(() => validateRelativePath("src/../../secret")).toThrow();
    expect(() => validateRelativePath("src/main.ts")).not.toThrow();
  });

  it("requires explicit permissions", () => {
    const context = { sessionId: "s1", approved: new Set(["read_project" as const]) };
    expect(() => requirePermission(context, "read_project")).not.toThrow();
    expect(() => requirePermission(context, "write_project")).toThrow();
  });
});
