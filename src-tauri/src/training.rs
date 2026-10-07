use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
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

#[derive(Debug, Serialize, Clone)]
pub struct TrainingProgress { pub phase: String, pub completed: u64, pub total: u64, pub percent: u8 }

#[derive(Debug, Serialize, Clone)]
pub struct TrainingJobStatus {
    pub job_id: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub progress: Option<TrainingProgress>,
}

#[derive(Clone, Default)]
pub struct TrainingState(pub Arc<Mutex<std::collections::HashMap<String, TrainingJobStatus>>>);

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

fn parse_progress_line(line: &str, job_id: &str) -> Option<TrainingProgress> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("status")?.as_str()? != "progress" || value.get("jobId")?.as_str()? != job_id {
        return None;
    }
    let phase = value.get("phase")?.as_str()?.to_string();
    let completed = value.get("completed")?.as_u64()?;
    let total = value.get("total")?.as_u64()?;
    let percent = value.get("percent")?.as_u64()?;
    if total == 0 || completed > total || percent > 100 {
        return None;
    }
    Some(TrainingProgress { phase, completed, total, percent: percent as u8 })
}

fn set_progress(state: &Arc<Mutex<std::collections::HashMap<String, TrainingJobStatus>>>, job_id: &str, progress: TrainingProgress) {
    if let Ok(mut jobs) = state.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            if job.status == "running" {
                job.progress = Some(progress);
            }
        }
    }
}

fn python_executable() -> String {
    std::env::var("WOHO_TRAINING_PYTHON").unwrap_or_else(|_| {
        if cfg!(windows) { "python.exe".into() } else { "python3".into() }
    })
}

#[tauri::command]
pub fn run_training(app: tauri::AppHandle, request: TrainingRunRequest) -> Result<TrainingRunResult, String> {
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

    let dataset_arg = dataset.to_string_lossy().into_owned();
    let output_arg = output.to_string_lossy().into_owned();
    let rank_arg = request.rank.to_string();
    let alpha_arg = request.alpha.to_string();
    let dropout_arg = request.dropout.to_string();
    let epochs_arg = request.epochs.to_string();
    let learning_rate_arg = request.learning_rate.to_string();

    let args = [
        "-m", "woho_training",
        "--job-id", request.job_id.as_str(),
        "--base-model", request.base_model.as_str(),
        "--dataset-manifest", dataset_arg.as_str(),
        "--output-dir", output_arg.as_str(),
        "--rank", rank_arg.as_str(),
        "--alpha", alpha_arg.as_str(),
        "--dropout", dropout_arg.as_str(),
        "--epochs", epochs_arg.as_str(),
        "--learning-rate", learning_rate_arg.as_str(),
        "--quantization", request.quantization.as_str(),
    ];

    if args.len() > MAX_ARGS {
        return Err("Training command has too many arguments".into());
    }

    let state = app.state::<TrainingState>();
    {
        let mut jobs = state.0.lock().map_err(|_| "Training state lock failed".to_string())?;
        if let Some(existing) = jobs.get(&request.job_id) {
            if existing.status == "running" { return Err("Training job is already running".into()); }
        }
        jobs.insert(request.job_id.clone(), TrainingJobStatus { job_id: request.job_id.clone(), status: "running".into(), exit_code: None, stdout: String::new(), stderr: String::new(), progress: None });
    }

    let resource_dir = app.path().resource_dir().map_err(|error| format!("Unable to resolve resource directory: {error}"))?;
    let python_path = resource_dir.to_string_lossy().into_owned();

    let mut child = Command::new(&executable)
        .args(args)
        .env("PYTHONPATH", &python_path)
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start QLoRA runtime: {error}"))?;

    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_reader = child.stdout.take().ok_or_else(|| "Training stdout pipe unavailable".to_string())?;
    let stderr_reader = child.stderr.take().ok_or_else(|| "Training stderr pipe unavailable".to_string())?;

    fn drain_output<R: Read + Send + 'static>(
        mut reader: R,
        overflow: Arc<AtomicBool>,
        progress_state: Option<Arc<Mutex<std::collections::HashMap<String, TrainingJobStatus>>>>,
        job_id: Option<String>,
    ) -> thread::JoinHandle<Result<Vec<u8>, String>> {
        thread::spawn(move || {
            let mut output = Vec::new();
            let mut pending = String::new();
            let mut buffer = [0u8; 32 * 1024];
            loop {
                let size = reader.read(&mut buffer).map_err(|error| format!("Training output read failed: {error}"))?;
                if size == 0 { break; }
                if output.len().saturating_add(size) <= MAX_OUTPUT_BYTES {
                    output.extend_from_slice(&buffer[..size]);
                } else {
                    overflow.store(true, Ordering::SeqCst);
                    continue;
                }
                if let (Some(state), Some(job_id)) = (&progress_state, &job_id) {
                    pending.push_str(&String::from_utf8_lossy(&buffer[..size]));
                    while let Some(index) = pending.find('\n') {
                        let line = pending[..index].trim_end_matches('\r').to_string();
                        pending.drain(..=index);
                        if let Some(progress) = parse_progress_line(&line, job_id) {
                            set_progress(state, job_id, progress);
                        }
                    }
                }
            }
            if let (Some(state), Some(job_id)) = (&progress_state, &job_id) {
                if let Some(progress) = parse_progress_line(pending.trim(), job_id) {
                    set_progress(state, job_id, progress);
                }
            }
            Ok(output)
        })
    }

    let progress_state = Arc::clone(&state.0);
    let stdout_handle = drain_output(stdout_reader, Arc::clone(&overflow), Some(progress_state), Some(request.job_id.clone()));
    let stderr_handle = drain_output(stderr_reader, Arc::clone(&overflow), None, None);
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(MAX_RUNTIME_SECS) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("QLoRA training exceeded the runtime limit".into());
        }
        if overflow.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_handle.join();
            let _ = stderr_handle.join();
            return Err("Training output exceeds the size limit".into());
        }
        if let Some(status) = child.try_wait().map_err(|error| format!("Training process failed: {error}"))? {
            let stdout = stdout_handle.join().map_err(|_| "Training stdout reader failed".to_string())??;
            let stderr = stderr_handle.join().map_err(|_| "Training stderr reader failed".to_string())??;
            let progress = {
                let jobs = state.0.lock().map_err(|_| "Training state lock failed".to_string())?;
                jobs.get(&request.job_id).and_then(|job| job.progress.clone())
            };
            let result = TrainingRunResult {
                job_id: request.job_id,
                status: if status.success() { "completed" } else { "failed" },
                exit_code: status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            };
            if let Ok(mut jobs) = state.0.lock() {
                jobs.insert(result.job_id.clone(), TrainingJobStatus { job_id: result.job_id.clone(), status: result.status.into(), exit_code: Some(result.exit_code), stdout: result.stdout.clone(), stderr: result.stderr.clone(), progress });
            }
            return Ok(result);
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
    fn parses_valid_progress_event() {
        let progress = parse_progress_line(
            r#"{"status":"progress","jobId":"job-1","phase":"training","completed":4,"total":10,"percent":40}"#,
            "job-1",
        ).unwrap();
        assert_eq!(progress.phase, "training");
        assert_eq!(progress.completed, 4);
        assert_eq!(progress.percent, 40);
    }

    #[test]
    fn rejects_invalid_progress_event() {
        assert!(parse_progress_line(
            r#"{"status":"progress","jobId":"job-1","phase":"training","completed":11,"total":10,"percent":110}"#,
            "job-1",
        ).is_none());
        assert!(parse_progress_line(
            r#"{"status":"progress","jobId":"other","phase":"training","completed":1,"total":1,"percent":100}"#,
            "job-1",
        ).is_none());
    }

    #[test]
    fn keeps_relative_training_paths_inside_root() {
        let root = Path::new("training");
        assert!(relative_path(root, "dataset/manifest.jsonl", "dataset").is_ok());
        assert!(relative_path(root, "../outside.jsonl", "dataset").is_err());
        assert!(relative_path(root, "/absolute/path", "dataset").is_err());
    }
}


#[tauri::command]
pub fn training_job_status(
    state: tauri::State<'_, TrainingState>,
    job_id: String,
) -> Result<TrainingJobStatus, String> {
    safe_component(&job_id, "training job id", 128)?;
    let jobs = state.0.lock().map_err(|_| "Training state lock failed".to_string())?;
    jobs.get(&job_id).cloned().ok_or_else(|| "Training job not found".into())
}

#[tauri::command]
pub fn training_job_clear(
    state: tauri::State<'_, TrainingState>,
    job_id: String,
) -> Result<(), String> {
    safe_component(&job_id, "training job id", 128)?;
    let mut jobs = state.0.lock().map_err(|_| "Training state lock failed".to_string())?;
    jobs.remove(&job_id);
    Ok(())
}
