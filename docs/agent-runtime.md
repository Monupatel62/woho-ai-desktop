# Agent Runtime

The desktop runtime uses a strict process boundary:

```
UI
  -> Tauri invoke(agent_chat)
  -> Node agent bridge
  -> @woho/agents AgentRuntime
  -> @woho/core AIClient
  -> llama.cpp provider
  -> verified GGUF model
```

## Development runtime

The bridge worker is `src/agent/agent-bridge.mjs`. It is started by the Tauri command with Node during development.

Managed runtime files are expected under the Tauri app-data directory:

- `runtimes/llama/llama-cli.exe` on Windows
- `models/<model-id>.gguf`

The model manager ships a trusted manifest. The current development model is `qwen3-0.6b-q4_0`, backed by the `ggml-org/Qwen3-0.6B-GGUF` Q4_0 artifact. Installation is restricted to the bundled HTTPS source, bounded to 512 MiB, downloaded to a unique temporary file, SHA-256 verified, flushed, and atomically renamed into the managed model directory.

Chat requests verify the installed model against the pinned size and SHA-256 before starting llama.cpp. Model binaries are never committed to Git.

## Production

The Node bridge will be packaged as a Tauri sidecar so users do not need to install Node separately. Tauri supports embedding external binaries/sidecars and resolves platform-specific target binaries during bundling. See the official Tauri sidecar documentation.

The native llama.cpp runtime is also expected to be installed into the managed runtime directory by a later production-runtime step. The model manager intentionally downloads model data after installation rather than embedding hundreds of megabytes in the installer.
