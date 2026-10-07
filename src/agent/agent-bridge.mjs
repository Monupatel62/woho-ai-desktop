import { createInterface } from "node:readline";
import { existsSync, statSync } from "node:fs";
import { isAbsolute, resolve, relative, sep } from "node:path";
import { spawn } from "node:child_process";
import { createAI } from "@woho/core";
import { AgentRuntime } from "@woho/agents";

const MAX_INPUT_BYTES = 64 * 1024;
const MAX_OUTPUT_BYTES = 4 * 1024 * 1024;
const MODEL_ID = /^[a-z0-9][a-z0-9._-]{0,127}$/;

function inside(root, candidate) {
  const r = resolve(root);
  const c = resolve(candidate);
  const rel = relative(r, c);
  return rel === "" || (rel !== ".." && !rel.startsWith(".." + sep) && !isAbsolute(rel));
}

function boundedText(value, maxBytes, label) {
  if (typeof value !== "string") throw new Error(label + " must be a string");
  if (Buffer.byteLength(value, "utf8") > maxBytes) throw new Error(label + " exceeds size limit");
  return value;
}

function buildPrompt(messages) {
  return messages
    .map((message) => {
      const role = message.role.toUpperCase();
      return role + ":\n" + message.content;
    })
    .join("\n\n") + "\n\nASSISTANT:\n";
}

function createLlamaProvider() {
  return {
    name: "llama.cpp",
    async chat(request) {
      const runtimeDir = process.env.WOHO_LLAMA_RUNTIME_DIR;
      const modelDir = process.env.WOHO_MODEL_DIR;
      const executable = process.env.WOHO_LLAMA_EXECUTABLE;
      if (!runtimeDir || !modelDir || !executable) {
        throw new Error("Local llama.cpp runtime is not configured");
      }
      if (!isAbsolute(runtimeDir) || !isAbsolute(modelDir) || !isAbsolute(executable)) {
        throw new Error("Local runtime paths must be absolute");
      }
      if (!inside(runtimeDir, executable) || !inside(modelDir, modelDir)) {
        throw new Error("Local runtime path escaped its configured root");
      }
      if (!existsSync(executable) || !statSync(executable).isFile()) {
        throw new Error("llama.cpp executable is missing");
      }
      const modelId = request.model ?? "";
      if (!MODEL_ID.test(modelId)) throw new Error("Invalid local model id");
      const modelPath = resolve(modelDir, modelId + ".gguf");
      if (!inside(modelDir, modelPath) || !existsSync(modelPath) || !statSync(modelPath).isFile()) {
        throw new Error("Local GGUF model is not installed: " + modelId);
      }

      const prompt = boundedText(buildPrompt(request.messages), MAX_INPUT_BYTES, "Prompt");
      const context = String(Number.isInteger(Number(process.env.WOHO_CONTEXT_TOKENS)) ? Number(process.env.WOHO_CONTEXT_TOKENS) : 8192);
      const temperature = String(Number.isFinite(Number(process.env.WOHO_TEMPERATURE)) ? Number(process.env.WOHO_TEMPERATURE) : 0.2);

      return await new Promise((resolvePromise, reject) => {
        const child = spawn(executable, [
          "-m", modelPath,
          "-p", prompt,
          "-n", "512",
          "-c", context,
          "--temp", temperature,
        ], { shell: false, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });

        let stdout = "";
        let stderr = "";
        let outputBytes = 0;
        const append = (chunk, target) => {
          outputBytes += chunk.byteLength;
          if (outputBytes > MAX_OUTPUT_BYTES) {
            child.kill();
            reject(new Error("llama.cpp output exceeds size limit"));
            return;
          }
          if (target === "stdout") stdout += chunk.toString("utf8");
          else stderr += chunk.toString("utf8");
        };
        child.stdout.on("data", (chunk) => append(chunk, "stdout"));
        child.stderr.on("data", (chunk) => append(chunk, "stderr"));
        child.on("error", (error) => reject(new Error("Failed to start llama.cpp: " + error.message)));
        child.on("close", (code) => {
          if (code !== 0) {
            reject(new Error("llama.cpp exited with code " + String(code)));
            return;
          }
          resolvePromise({
            id: "llama-local",
            model: modelId,
            text: stdout.trim(),
            finishReason: "stop",
          });
        });
      });
    },
  };
}

async function main(request) {
  if (!request || typeof request !== "object") throw new Error("Invalid agent request");
  const conversationId = boundedText(request.conversationId ?? "", 256, "conversationId");
  const message = boundedText(request.message ?? "", MAX_INPUT_BYTES, "message");
  const modelId = boundedText(request.modelId ?? "", 128, "modelId");
  if (!conversationId || !message || !MODEL_ID.test(modelId)) throw new Error("Invalid agent request");
  if (request.runtime !== "local") throw new Error("Desktop agent bridge currently supports local runtime only");

  const ai = createAI({ provider: createLlamaProvider(), timeoutMs: 120_000, retries: 0 });
  const runtime = new AgentRuntime({
    maxConcurrency: 1,
    maxInputBytes: MAX_INPUT_BYTES,
    executionTimeoutMs: 125_000,
  });
  runtime.register({
    id: "desktop-assistant",
    name: "WoHo Desktop Assistant",
    role: "general",
    instructions: "You are WoHo AI Desktop. Answer clearly and helpfully. Do not claim to have performed actions you did not perform.",
    maxSteps: 1,
  });

  const result = await runtime.run(ai, {
    agent: "desktop-assistant",
    input: message,
    sessionId: conversationId,
  });
  return { text: result.text, modelId, runtime: "local" };
}

const input = createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of input) {
  if (!line.trim()) continue;
  try {
    const result = await main(JSON.parse(line));
    process.stdout.write(JSON.stringify({ ok: true, result }) + "\n");
  } catch (error) {
    process.stdout.write(JSON.stringify({
      ok: false,
      error: error instanceof Error ? error.message : "Agent bridge failed",
    }) + "\n");
  }
  break;
}
