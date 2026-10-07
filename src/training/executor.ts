import { validateQloraConfig } from "./qlora";
import type { TrainingJob } from "./job";

export interface TrainingCommand {
  executable: string;
  args: string[];
  cwd: string;
}

export function buildTrainingCommand(job: TrainingJob, pythonExecutable = "python"): TrainingCommand {
  validateQloraConfig(job.config);
  if (!/^[A-Za-z0-9._-]+$/.test(pythonExecutable)) throw new Error("Invalid training executable");
  return {
    executable: pythonExecutable,
    cwd: job.config.outputDir,
    args: [
      "-m", "woho_training",
      "--base-model", job.config.baseModel,
      "--dataset-manifest", job.config.datasetManifest,
      "--output-dir", job.config.outputDir,
      "--rank", String(job.config.rank),
      "--alpha", String(job.config.alpha),
      "--dropout", String(job.config.dropout),
      "--epochs", String(job.config.epochs),
      "--learning-rate", String(job.config.learningRate),
      "--quantization", job.config.quantization,
    ],
  };
}

export function validateTrainingCommand(command: TrainingCommand): void {
  if (!command.executable || command.executable.includes("/") || command.executable.includes("\\") || command.executable.includes("..")) {
    throw new Error("Training executable must be a trusted command name");
  }
  if (!command.cwd || command.cwd.includes("\0")) throw new Error("Invalid training working directory");
  if (command.args.length > 64) throw new Error("Training command has too many arguments");
}
