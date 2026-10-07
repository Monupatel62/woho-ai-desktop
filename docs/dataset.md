# WoHo Dataset Architecture

WoHo training data uses a versioned JSON contract so datasets can evolve without silently changing meaning.

## Schema v1

A dataset contains `schemaVersion: 1` and ordered `examples`. Each example has a stable ID and ordered messages. Roles are `system`, `user`, or `assistant`. Optional string metadata carries provenance such as source or language.

## JSONL

Training pipelines can consume deterministic JSONL: one example per line, preserving source order. The loader validates the full dataset before export.

## Limits

The contract rejects unsupported versions, invalid IDs, unknown roles, empty content, messages over 64 KiB, more than 128 messages per example, metadata keys over 128 bytes, metadata values over 1 KiB, and dataset JSON larger than 16 MiB.

External ingestion should add signed manifests and content hashes before accepting untrusted datasets.
