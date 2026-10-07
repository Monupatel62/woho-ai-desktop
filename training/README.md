# Training

Training is designed for free/low-cost accelerator environments such as temporary Colab/Kaggle sessions.

The repository stores configuration and scripts, not model checkpoints or credentials.

Recommended flow:
1. prepare a versioned dataset manifest
2. run QLoRA training
3. evaluate against the benchmark
4. export/merge adapter
5. quantize to GGUF
6. publish a signed model manifest
