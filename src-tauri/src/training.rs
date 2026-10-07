use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tauri::Manager;

const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_ARGS: usize = 64;
const MAX_RUNTIME_SECS: u64 = 60 * 60;

#[derive(Debug, Deserialize)]
pub struct TrainingRunRequest {
    #[serde(rename = "jobId")]
    pub job_id: String,
    #[serde(rename = "baseModel")]
    pub base_model: String,
    #[serde(rename = "datasetManifest")]
    pub dataset_manifest: String,
    #[serde(rename = "outputDir")]
    pub output_dir: String,
    pub rank: u32,
    pub alpha: u32,
    pub dropout: f64,
    pub epochs: u32,
    #[serde(rename = "learningRate")]
    pub learning_rate: f64,
    pub quantization: String,
}

#[derive(Debug, Serialize)]
pub struct TrainingRunResult {
    pub job_id: String,
    pub status: &'static str,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

fn safe_component(value: &str, label: &str, max: usize) -> Result<(), String> {
    if value.is_empty() || value.len() > max || value == "." || value == ".."
        || value.contains('\0') || value.contains('/') || value.contains('\\')
    {
        return Err(format!("Invalid {label}"));
    }
    Ok(())
}

fn relative_path(root: &Path, value: &str, label: &str) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains('\0') {
        return Err(format!("Invalid {label}"));
    }
    let candidate = root.join(value);
    let normalized = candidate.components().collect::<PathBuf>();
    if !normalized.starts_with(root) {
        return Err(format!("{label} escapes the training directory"));
    }
    Ok(normalized)
}

fn append_bounded(target: &mut Vec<u8>, chunk: &[u8]) -> Result<(), String> {
    if target.len().saturating_add(chunk.len()) > MAX_OUTPUT_BYTES {
        return Err("Training output exceeds the size limit".into());
    }
    target.extend_from_slice(chunk);
    Ok(())
}

fn python_executable() -> String {
    std::env::var("WOHO_TRAINING_PYTHON").unwrap_or_else(|_| {
        if cfg!(windows) { "python.exe".into() } else { "python3".into() }
    })
}

#[tauri::command]
pub fn run_training(app: &tauri::AppHandle, request: TrainingRunRequest) -> Result<TrainingRunResult, String> {
    safe_component(&request.job_id, "training job id", 128)?;
    safe_component(&request.base_model, "base model", 256)?;
    if request.rank == 0 || request.rank > 256 || request.alpha == 0 || request.alpha > 1024 {
        return Err("Invalid LoRA rank or alpha".into());
    }
    if !request.dropout.is_finite() || !(0.0..=1.0).contains(&request.dropout) {
        return Err("Invalid LoRA dropout".into());
    }
    if request.epochs == 0 || request.epochs > 100 || !request.learning_rate.is_finite()
        || request.learning_rate <= 0.0 || request.learning_rate > 1.0
    {
        return Err("Invalid training hyperparameters".into());
    }
    if request.quantization != "4bit" && request.quantization != "8bit" {
        return Err("Invalid training quantization".into());
    }

    let root = app.path().app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?
        .join("training");
    fs::create_dir_all(&root)
        .map_err(|error| format!("Unable to create training directory: {error}"))?;

    let dataset = relative_path(&root, &request.dataset_manifest, "dataset manifest")?;
    let output = relative_path(&root, &request.output_dir, "output directory")?;
    if !dataset.is_file() {
        return Err("Training dataset manifest does not exist".into());
    }
    fs::create_dir_all(&output)
        .map_err(|error| format!("Unable to create training output directory: {error}"))?;

    let executable = python_executable();
    if executable.contains('/') || executable.contains('\\') || executable.contains("..") {
        return Err("Training Python executable must be a trusted command name".into());
    }

    let args = [
        "-m", "woho_training",
        "--base-model", request.base_model.as_str(),
        "--dataset-manifest", dataset.to_string_lossy().as_ref(),
        "--output-dir", output.to_string_lossy().as_ref(),
        "--rank", &request.rank.to_string(),
        "--alpha", &request.alpha.to_string(),
        "--dropout", &request.dropout.to_string(),
        "--epochs", &request.epochs.to_string(),
        "--learning-rate", &request.learning_rate.to_string(),
        "--quantization", request.quantization.as_str(),
    ];

    if args.len() > MAX_ARGS {
        return Err("Training command has too many arguments".into());
    }

    let mut child = Command::new(&executable)
        .args(args)
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start QLoRA runtime: {error}"))?;

    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(MAX_RUNTIME_SECS) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("QLoRA training exceeded the runtime limit".into());
        }
        if let Some(status) = child.try_wait().map_err(|error| format!("Training process failed: {error}"))? {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut stream) = child.stdout.take() {
                let mut buffer = [0u8; 32 * 1024];
                while let Ok(size) = stream.read(&mut buffer) {
                    if size == 0 { break; }
                    append_bounded(&mut stdout, &buffer[..size])?;
                }
            }
            if let Some(mut stream) = child.stderr.take() {
                let mut buffer = [0u8; 32 * 1024];
                while let Ok(size) = stream.read(&mut buffer) {
                    if size == 0 { break; }
                    append_bounded(&mut stderr, &buffer[..size])?;
                }
            }
            return Ok(TrainingRunResult {
                job_id: request.job_id,
                status: if status.success() { "completed" } else { "failed" },
                exit_code: status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            });
        }
        thread::sleep(Duration::from_millis(100));
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_components() {
        assert!(safe_component("../python", "executable", 128).is_err());
        assert!(safe_component("python", "executable", 128).is_ok());
        assert!(safe_component("model/name", "base model", 256).is_err());
    }

    #[test]
    fn keeps_relative_training_paths_inside_root() {
        let root = Path::new("training");
        assert!(relative_path(root, "dataset/manifest.jsonl", "dataset").is_ok());
        assert!(relative_path(root, "../outside.jsonl", "dataset").is_err());
        assert!(relative_path(root, "/absolute/path", "dataset").is_err());
    }
}
