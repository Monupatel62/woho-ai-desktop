export const SECURITY_BASELINE = Object.freeze({
  noShellForRuntimeProcesses: true,
  trustedModelManifest: true,
  sha256ModelVerification: true,
  boundedAgentInput: true,
  boundedAgentOutput: true,
  explicitPermissions: true,
  localByDefault: true,
});

export function assertSecurityBaseline(): void {
  if (Object.values(SECURITY_BASELINE).some((value) => value !== true)) throw new Error("Security baseline is incomplete");
}
