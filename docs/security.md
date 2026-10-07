# Security Baseline

WoHo AI Desktop is local-first and permission-gated.

Rules:
- project paths must stay inside the selected workspace
- tool execution requires an explicit permission
- model downloads require HTTPS from the bundled trusted host and SHA-256 verification
- cloud runtime is opt-in
- secrets must never be stored in source-controlled files
- large models/datasets/checkpoints stay outside Git
- every privileged operation must be attributable to a session

## Model manager boundary

- The UI sends only a model ID; it cannot choose an arbitrary download URL.
- Rust resolves that ID against the bundled manifest.
- Manifest sources are HTTPS on `huggingface.co`, with a pinned SHA-256 and exact byte size.
- Downloads are bounded to 512 MiB and streamed through a temporary file.
- The temporary file is created with `create_new`, flushed, SHA-256 verified, then atomically renamed.
- Existing files are never silently overwritten.
- Model IDs and managed filenames are constrained to the safe local-runtime form.
- Chat refuses to invoke llama.cpp unless the selected model passes the same SHA-256 verification.

## Agent / llama.cpp boundary

- The UI can request only the Tauri `agent_chat` command.
- The Tauri command launches the controlled agent bridge; it does not accept an arbitrary executable path from the UI.
- llama.cpp is launched without a shell and with bounded prompt/output sizes.
- Production packaging will ship the Node agent bridge as a Tauri sidecar; development can use the repository worker with Node.
