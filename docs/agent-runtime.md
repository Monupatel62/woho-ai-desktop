# Agent Runtime

The desktop runtime uses a strict process boundary:

```
UI
  -> Tauri invoke(agent_chat)
  -> Node agent bridge
  -> @woho/agents AgentRuntime
  -> @woho/core AIClient
  -> llama.cpp provider
  -> GGUF model
```

## Development runtime

The bridge worker is `src/agent/agent-bridge.mjs`. It is started by the Tauri command with Node during development.

Managed runtime files are expected under the Tauri app-data directory:

- `runtimes/llama/llama-cli.exe` on Windows
- `models/<model-id>.gguf`

The model id is validated before the path is resolved. The worker uses llama.cpp's `-m`, `-p`, `-n`, `-c` and `--temp` options for bounded non-interactive generation. llama.cpp requires GGUF model files for local inference. citeturn1search1turn1search0

## Production

The Node bridge will be packaged as a Tauri sidecar so users do not need to install Node separately. Tauri supports embedding external binaries/sidecars and resolves platform-specific target binaries during bundling. citeturn2search0turn2search5

This phase intentionally does not commit model binaries or native llama.cpp binaries to Git.
