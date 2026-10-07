# 20-Phase Status

| Phase | Foundation in this repository |
|---|---|
| 1 | Tauri Windows desktop shell |
| 2 | @woho/agents runtime bridge implemented |
| 3 | Managed llama.cpp process bridge implemented |
| 4 | **Complete — Qwen3 0.6B GGUF trusted model integration with real local chat** |
| 5 | **Complete — versioned dataset schema, validation, deterministic JSONL export, provenance metadata, and regression tests** |
| 6 | **Complete — reproducible QLoRA training configuration with pinned defaults and validation** |
| 7 | **Complete — validated QLoRA hyperparameter contract** |
| 8 | **Complete — deterministic exact-match benchmark scorer and summary** |
| 9 | **Complete — GGUF artifact identity, size, digest, and filename contract** |
| 10 | **Complete — runtime registry with duplicate-registration protection** |
| 11 | **Complete — validated tool registry with duplicate protection and risk metadata** |
| 12 | **Complete — explicit permission context and enforcement boundary** |
| 13 | **Complete — bounded in-memory memory store contract** |
| 14 | **Complete — workspace validation and write-permission boundary** |
| 15 | **Complete — permission-gated coding-agent runtime orchestration boundary** |
| 16 | **Complete — optional cloud runtime provider abstraction** |
| 17 | **Complete — trusted manifest, list/install/verify/remove, atomic install, exact size + SHA-256 verification, and desktop UI** |
| 18 | **Complete — security baseline assertions covering runtime/model/input/permission boundaries** |
| 19 | **Complete — architecture regression coverage added alongside existing model/dataset tests** |
| 20 | **Complete — Windows MSI/NSIS production pipeline with bundled Node SEA agent + verified llama.cpp CPU runtime** |

## Model policy

Model binaries are never committed to Git. The desktop app downloads the pinned development GGUF into managed app-data storage, verifies its exact size and SHA-256, and only then exposes it to the local agent runtime.


## Windows production release

The release workflow builds on `windows-latest`, creates the Node agent bridge as a single executable, verifies and bundles llama.cpp b11430 CPU x64, and produces MSI/NSIS artifacts. Runtime resources are generated only in CI and are excluded from Git. Users do not need Node.js or a separate llama.cpp installation.
