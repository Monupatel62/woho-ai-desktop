import { build } from "esbuild";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const buildDir = resolve(root, ".build");
const agentDir = resolve(root, "src-tauri", "resources", "agent");
const bundle = resolve(buildDir, "agent-bridge.cjs");
const seaConfig = resolve(buildDir, "sea-config.json");
const seaBlob = resolve(buildDir, "sea-prep.blob");
const output = resolve(agentDir, "woho-agent.exe");

rmSync(buildDir, { recursive: true, force: true });
mkdirSync(buildDir, { recursive: true });
mkdirSync(agentDir, { recursive: true });

await build({
  entryPoints: [resolve(root, "src", "agent", "agent-bridge.mjs")],
  outfile: bundle,
  bundle: true,
  platform: "node",
  format: "cjs",
  target: "node20",
  sourcemap: false,
  legalComments: "none",
  minify: false,
});

writeFileSync(
  seaConfig,
  JSON.stringify({
    main: bundle,
    output: seaBlob,
    disableExperimentalSEAWarning: true,
    useSnapshot: false,
    useCodeCache: true,
  }),
);

execFileSync(process.execPath, ["--experimental-sea-config", seaConfig], {
  cwd: root,
  stdio: "inherit",
});

copyFileSync(process.execPath, output);

const postject = resolve(
  root,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "postject.cmd" : "postject",
);
execFileSync(
  postject,
  [
    output,
    "NODE_SEA_BLOB",
    seaBlob,
    "--sentinel-fuse",
    "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2",
  ],
  { cwd: root, stdio: "inherit", shell: process.platform === "win32" },
);

const stat = readFileSync(output);
if (stat.length < 10 * 1024 * 1024) {
  throw new Error("Agent sidecar is unexpectedly small");
}

console.log(`Built production agent sidecar: ${output}`);
