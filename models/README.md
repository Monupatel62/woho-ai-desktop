# Models

Large model binaries are never committed to Git.

A model installation must:
- use HTTPS or an explicit local source
- verify SHA-256 before activation
- match a trusted model manifest
- remain outside the application source tree

GGUF is the target local format for llama.cpp.
