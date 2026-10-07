# 20-Phase Status

| Phase | Foundation in this repository |
|---|---|
| 1 | Tauri Windows desktop shell |
| 2 | @woho/agents runtime bridge implemented |
| 3 | Managed llama.cpp process bridge implemented |
| 4 | Qwen3 0.6B GGUF trusted model integration |
| 5 | Dataset example schema |
| 6 | Reproducible training configuration |
| 7 | QLoRA configuration |
| 8 | Deterministic benchmark scorer |
| 9 | GGUF model format contract |
| 10 | Local runtime abstraction |
| 11 | Tool registry |
| 12 | Permission engine |
| 13 | Memory store contract |
| 14 | Project workspace boundary |
| 15 | Coding-agent orchestration boundary |
| 16 | Optional cloud provider |
| 17 | Verified model manager with install/remove/verify commands |
| 18 | Security validation baseline |
| 19 | Regression tests |
| 20 | Windows MSI/NSIS bundle configuration |

## Model policy

Model binaries are never committed to Git. The desktop app downloads the pinned development GGUF into managed app-data storage, verifies its exact size and SHA-256, and only then exposes it to the local agent runtime.
