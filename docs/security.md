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
