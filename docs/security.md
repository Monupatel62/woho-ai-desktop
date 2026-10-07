# Security Baseline

WoHo AI Desktop is local-first and permission-gated.

Rules:
- project paths must stay inside the selected workspace
- tool execution requires an explicit permission
- model downloads require HTTPS and SHA-256 verification
- cloud runtime is opt-in
- secrets must never be stored in source-controlled files
- large models/datasets/checkpoints stay outside Git
- every privileged operation must be attributable to a session


## Agent / llama.cpp boundary

- The UI can request only the Tauri `agent_chat` command.
- The Tauri command launches the controlled agent bridge; it does not accept an arbitrary executable path from the UI.
- The bridge validates model IDs and resolves GGUF files only below the managed model directory.
- llama.cpp is launched without a shell and with bounded prompt/output sizes.
- Production packaging will ship the Node agent bridge as a Tauri sidecar; development can use the repository worker with Node.
