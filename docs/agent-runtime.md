# Agent Runtime

The desktop runtime uses a strict process boundary:

```
UI
  -> Tauri invoke(agent_chat)
  -> Node agent sidecar (production) / Node worker (development)
  -> @woho/agents AgentRuntime
  -> @woho/core AIClient
  -> llama.cpp
  -> verified GGUF model
```

## Production Windows runtime

The Windows release pipeline builds the Node agent bridge into a single executable using Node's Single Executable Application (SEA) support. The resulting `woho-agent.exe` is bundled as an application resource, so users do not need Node.js installed.

The release pipeline also downloads a pinned llama.cpp Windows x64 CPU archive, verifies its SHA-256, extracts the runtime into the build-only resource directory, and bundles it with the installer. The application resolves the packaged `llama/` resource at runtime; the user does not need to install llama.cpp separately.

The current pinned runtime is llama.cpp `b11430` with archive SHA-256:

`b608455b0109793f774537d63d15d4cf2098ddbc5b200ebc9a648e1d85369666`

The current release is CPU-only. GPU-specific runtimes can be added later as hardware-targeted bundles.

## Development runtime

The repository worker remains `src/agent/agent-bridge.mjs` for development. If the packaged agent resource is unavailable, Tauri falls back to this worker and the configured Node executable.

Development llama.cpp remains configurable through the managed app-data runtime directory and environment overrides.

## Model runtime

Models are downloaded after installation into managed app-data storage. They are never committed to Git or embedded in the installer. The model manager verifies the exact byte size and SHA-256 before a model is exposed to the agent.

## Security boundary

Production paths are resolved from Tauri's resource directory rather than accepted from the UI. The UI can provide only a trusted model ID. The agent bridge receives managed runtime/model paths from Rust and launches llama.cpp without a shell.
