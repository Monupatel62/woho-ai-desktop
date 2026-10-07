from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
from typing import Any

MAX_MANIFEST_BYTES = 8 * 1024 * 1024
MAX_DATASET_RECORDS = 1_000_000
ALLOWED_QUANTIZATION = {"4bit", "8bit"}
PROGRESS_VERSION = 1


def _bounded_path(value: str, root: Path, label: str) -> Path:
    if not value or "\\x00" in value:
        raise ValueError(f"invalid {label}")
    candidate = (root / value).resolve()
    root_resolved = root.resolve()
    if candidate != root_resolved and root_resolved not in candidate.parents:
        raise ValueError(f"{label} escapes training directory")
    return candidate


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(path: Path) -> list[dict[str, Any]]:
    if path.stat().st_size > MAX_MANIFEST_BYTES:
        raise ValueError("dataset manifest exceeds size limit")
    records: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for index, line in enumerate(handle, 1):
            if index > MAX_DATASET_RECORDS:
                raise ValueError("dataset manifest exceeds record limit")
            line = line.strip()
            if not line:
                continue
            item = json.loads(line)
            if not isinstance(item, dict):
                raise ValueError(f"manifest line {index} must be an object")
            text = item.get("text")
            if not isinstance(text, str) or not text.strip():
                raise ValueError(f"manifest line {index} requires non-empty text")
            records.append(item)
    if not records:
        raise ValueError("dataset manifest is empty")
    return records


def validate_request(args: argparse.Namespace, root: Path) -> tuple[Path, Path]:
    if not args.base_model or "/" in args.base_model or "\\\\" in args.base_model or ".." in args.base_model:
        raise ValueError("invalid base model")
    if args.rank < 1 or args.rank > 256 or args.alpha < 1 or args.alpha > 1024:
        raise ValueError("invalid LoRA rank or alpha")
    if not 0.0 <= args.dropout <= 1.0:
        raise ValueError("invalid dropout")
    if args.epochs < 1 or args.epochs > 100:
        raise ValueError("invalid epochs")
    if not 0.0 < args.learning_rate <= 1.0:
        raise ValueError("invalid learning rate")
    if args.quantization not in ALLOWED_QUANTIZATION:
        raise ValueError("invalid quantization")
    dataset = _bounded_path(args.dataset_manifest, root, "dataset manifest")
    output = _bounded_path(args.output_dir, root, "output directory")
    if not dataset.is_file():
        raise ValueError("dataset manifest does not exist")
    output.mkdir(parents=True, exist_ok=True)
    return dataset, output


def write_training_plan(args: argparse.Namespace, dataset: Path, output: Path, records: list[dict[str, Any]]) -> Path:
    plan = {
        "schemaVersion": 1,
        "jobId": args.job_id,
        "baseModel": args.base_model,
        "datasetSha256": _sha256(dataset),
        "datasetRecords": len(records),
        "rank": args.rank,
        "alpha": args.alpha,
        "dropout": args.dropout,
        "epochs": args.epochs,
        "learningRate": args.learning_rate,
        "quantization": args.quantization,
        "status": "validated",
        "progressVersion": PROGRESS_VERSION,
        "progress": {"phase": "validation", "completed": 0, "total": 1, "percent": 0},
    }
    target = output / "training-plan.json"
    target.write_text(json.dumps(plan, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description="WoHo bounded QLoRA training runtime")
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--base-model", required=True)
    parser.add_argument("--dataset-manifest", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--rank", type=int, required=True)
    parser.add_argument("--alpha", type=int, required=True)
    parser.add_argument("--dropout", type=float, required=True)
    parser.add_argument("--epochs", type=int, required=True)
    parser.add_argument("--learning-rate", type=float, required=True)
    parser.add_argument("--quantization", required=True)
    args = parser.parse_args()
    root = Path.cwd().resolve()
    try:
        dataset, output = validate_request(args, root)
        records = load_manifest(dataset)
        plan = write_training_plan(args, dataset, output, records)
        print(json.dumps({"status": "progress", "jobId": args.job_id, "phase": "validation", "completed": 1, "total": 1, "percent": 100}), flush=True)\n        print(json.dumps({"status": "validated", "jobId": args.job_id, "records": len(records), "plan": str(plan)}), flush=True)
        backend = os.environ.get("WOHO_TRAINING_BACKEND")
        if not backend:
            print(json.dumps({"status": "ready", "message": "Training backend not installed; validated plan only."}), flush=True)
            return 0
        raise RuntimeError("external training backend execution is not enabled by this contract")
    except Exception as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}), flush=True)
        return 1
