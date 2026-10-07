# Desktop Architecture Completion

The desktop repository now contains the implementation contracts for the full 20-phase roadmap.

## Training and evaluation
- Versioned dataset schema and deterministic JSONL export.
- Reproducible QLoRA configuration with validation.
- Deterministic benchmark scoring.
- GGUF artifact validation.

## Runtime and agent
- Local/cloud runtime abstraction.
- Runtime registry.
- Permission-gated coding-agent boundary.
- Optional cloud provider adapter.

## Tools, security, memory, workspace
- Tool registry with stable IDs and risk classification.
- Explicit permission context.
- Bounded in-memory memory contract.
- Workspace validation boundary.
- Security baseline assertions.

## Production boundary
The Windows pipeline builds the Node SEA agent, downloads a pinned llama.cpp CPU runtime, verifies its SHA-256, and compiles the Tauri application. Model binaries remain outside Git and are verified in managed app-data storage.

These are architecture/runtime foundations. A future phase can replace the in-memory implementations or provider adapters with persistent or vendor-specific implementations without changing the contracts.
