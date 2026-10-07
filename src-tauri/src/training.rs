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
const TRAINING_STATE_VERSION: u32 = 1;
const MAX_PERSISTED_JOBS: usize = 64;
const MAX_PERSISTED_OUTPUT_BYTES: usize = 64 * 1024;
const TRAINING_PLAN_SCHEMA_VERSION: u64 = 1;
const MAX_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainingProgress { pub phase: String, pub completed: u64, pub total: u64, pub percent: u8 }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainingJobStatus {
    pub job_id: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub progress: Option<TrainingProgress>,
    pub artifact: Option<TrainingArtifact>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainingArtifact {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub schema_version: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedTrainingState {
    version: u32,
    jobs: Vec<TrainingJobStatus>,
}

#[derive(Clone)]
pub struct TrainingState(
    pub Arc<Mutex<std::collections::HashMap<String, TrainingJobStatus>>>,
    Arc<PathBuf>,
    Arc<Mutex<()>>,
);

impl TrainingState {
    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let root = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Unable to resolve app data directory: {error}"))?
            .join("training");
        fs::create_dir_all(&root)
            .map_err(|error| format!("Unable to create training directory: {error}"))?;
        let path = root.join("jobs.json");
        let mut jobs = std::collections::HashMap::new();
        let mut recovered = false;
        if path.is_file() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("Unable to read persisted training state: {error}"))?;
            let persisted = decode_persisted_state(&bytes)?;
            for mut job in persisted.jobs.into_iter().take(MAX_PERSISTED_JOBS) {
                if job.status == "running" {
                    job.status = "failed".into();
                    job.stderr = "Training job interrupted by application restart".into();
                    recovered = true;
                }
                jobs.insert(job.job_id.clone(), job);
            }
        }
        let state = Self(Arc::new(Mutex::new(jobs)), Arc::new(path), Arc::new(Mutex::new(())));
        if recovered {
            state.persist()?;
        }
        Ok(state)
    }

    fn persisted_jobs(&self) -> Result<Vec<TrainingJobStatus>, String> {
        let jobs = self.0.lock().map_err(|_| "Training state lock failed".to_string())?;
        let mut values = jobs.values().cloned().collect::<Vec<_>>();
        values.sort_by(|a, b| a.job_id.cmp(&b.job_id));
        values.truncate(MAX_PERSISTED_JOBS);
        for job in &mut values {
            if job.stdout.len() > MAX_PERSISTED_OUTPUT_BYTES {
                job.stdout.truncate(MAX_PERSISTED_OUTPUT_BYTES);
            }
            if job.stderr.len() > MAX_PERSISTED_OUTPUT_BYTES {
                job.stderr.truncate(MAX_PERSISTED_OUTPUT_BYTES);
            }
        }
        Ok(values)
    }

    fn persist(&self) -> Result<(), String> {
        let _guard = self.2.lock().map_err(|_| "Training persistence lock failed".to_string())?;
        let snapshot = PersistedTrainingState {
            version: TRAINING_STATE_VERSION,
            jobs: self.persisted_jobs()?,
        };
        let bytes = encode_persisted_state(&snapshot)?;
        let path = self.1.as_ref();
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, bytes)
            .map_err(|error| format!("Unable to write training state: {error}"))?;
        fs::rename(&temp, path)
            .map_err(|error| format!("Unable to atomically replace training state: {error}"))?;
        Ok(())
    }
}

fn encode_persisted_state(state: &PersistedTrainingState) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(state)
        .map_err(|error| format!("Unable to encode training state: {error}"))
}

fn decode_persisted_state(bytes: &[u8]) -> Result<PersistedTrainingState, String> {
    let persisted: PersistedTrainingState = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid persisted training state: {error}"))?;
    if persisted.version != TRAINING_STATE_VERSION {
        return Err("Unsupported persisted training state version".into());
    }
    Ok(persisted)
}

fn sha256_file(path: &Path, max_bytes: u64) -> Result<(String, u64), String> {
    use sha2::{Digest, Sha256};
    let metadata = fs::metadata(path).map_err(|error| format!("Unable to inspect training artifact: {error}"))?;
    if metadata.len() > max_bytes { return Err("Training artifact exceeds the size limit".into()); }
    let mut file = fs::File::open(path).map_err(|error| format!("Unable to open training artifact: {error}"))?;
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| format!("Unable to read training artifact: {error}"))?;
        if read == 0 { break; }
        total = total.saturating_add(read as u64);
        if total > max_bytes { return Err("Training artifact exceeds the size limit".into()); }
        digest.update(&buffer[..read]);
    }
    Ok((format!("{:x}", digest.finalize()), total))
}

fn verify_training_artifact(root: &Path, output: &Path, job_id: &str, dataset: &Path) -> Result<TrainingArtifact, String> {
    let plan = output.join("training-plan.json");
    if !plan.is_file() { return Err("Training artifact training-plan.json is missing".into()); }
    let bytes = fs::read(&plan).map_err(|error| format!("Unable to read training plan: {error}"))?;
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES { return Err("Training artifact exceeds the size limit".into()); }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| format!("Training plan is not valid JSON: {error}"))?;
    if value.get("schemaVersion").and_then(serde_json::Value::as_u64) != Some(TRAINING_PLAN_SCHEMA_VERSION) { return Err("Unsupported training plan schema version".into()); }
    if value.get("jobId").and_then(serde_json::Value::as_str) != Some(job_id) { return Err("Training plan job ID does not match the requested job".into()); }
    let expected_dataset = value.get("datasetSha256").and_then(serde_json::Value::as_str).ok_or_else(|| "Training plan is missing datasetSha256".to_string())?;
    let (dataset_hash, _) = sha256_file(dataset, MAX_ARTIFACT_BYTES)?;
    if expected_dataset != dataset_hash { return Err("Training plan dataset hash does not match the source manifest".into()); }
    let (hash, size_bytes) = sha256_file(&plan, MAX_ARTIFACT_BYTES)?;
    let relative = plan.strip_prefix(root).map_err(|_| "Training artifact escaped training directory".to_string())?;
    Ok(TrainingArtifact { path: relative.to_string_lossy().into_owned(), sha256: hash, size_bytes, schema_version: TRAINING_PLAN_SCHEMA_VERSION })
}

#[derive(Debug, Serialize)]
pub struct TrainingRunResult {
    pub job_id: String,
    pub status: &'static str,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub artifact: Option<TrainingArtifact>,
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

fn set_progress(state: &TrainingState, job_id: &str, progress: TrainingProgress) {
    if let Ok(mut jobs) = state.0.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            if job.status == "running" {
                job.progress = Some(progress);
            }
        }
    }
    let _ = state.persist();
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
        jobs.insert(request.job_id.clone(), TrainingJobStatus { job_id: request.job_id.clone(), status: "running".into(), exit_code: None, stdout: String::new(), stderr: String::new(), progress: None, artifact: None });
    }
    if let Err(error) = state.persist() {
        if let Ok(mut jobs) = state.0.lock() {
            if let Some(job) = jobs.get_mut(&request.job_id) {
                job.status = "failed".into();
                job.stderr = format!("Unable to persist training start state: {error}");
            }
        }
        let _ = state.persist();
        return Err(format!("Unable to persist training start state: {error}"));
    }

    let resource_dir = match app.path().resource_dir() {
        Ok(path) => path,
        Err(error) => {
            if let Ok(mut jobs) = state.0.lock() {
                if let Some(job) = jobs.get_mut(&request.job_id) {
                    job.status = "failed".into();
                    job.stderr = format!("Unable to resolve resource directory: {error}");
                }
            }
            let _ = state.persist();
            return Err(format!("Unable to resolve resource directory: {error}"));
        }
    };
    let training_script = resource_dir.join("woho_training.py");
    if !training_script.is_file() {
        if let Ok(mut jobs) = state.0.lock() { if let Some(job) = jobs.get_mut(&request.job_id) { job.status = "failed".into(); job.stderr = "Bundled training runtime is missing".into(); } }
        let _ = state.persist();
        return Err("Bundled training runtime is missing".into());
    }
    let training_script_arg = training_script.to_string_lossy().into_owned();
    let mut child = match Command::new(&executable)
        .arg(training_script_arg.as_str())
        .args(args)
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            if let Ok(mut jobs) = state.0.lock() {
                if let Some(job) = jobs.get_mut(&request.job_id) {
                    job.status = "failed".into();
                    job.stderr = format!("Failed to start QLoRA runtime: {error}");
                }
            }
            let _ = state.persist();
            return Err(format!("Failed to start QLoRA runtime: {error}"));
        }
    };

    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_reader = match child.stdout.take() { Some(reader) => reader, None => { let _ = child.kill(); let _ = child.wait(); if let Ok(mut jobs) = state.0.lock() { if let Some(job) = jobs.get_mut(&request.job_id) { job.status = "failed".into(); job.stderr = "Training stdout pipe unavailable".into(); } } let _ = state.persist(); return Err("Training stdout pipe unavailable".into()); } };
    let stderr_reader = match child.stderr.take() { Some(reader) => reader, None => { let _ = child.kill(); let _ = child.wait(); if let Ok(mut jobs) = state.0.lock() { if let Some(job) = jobs.get_mut(&request.job_id) { job.status = "failed".into(); job.stderr = "Training stderr pipe unavailable".into(); } } let _ = state.persist(); return Err("Training stderr pipe unavailable".into()); } };

    fn drain_output<R: Read + Send + 'static>(
        mut reader: R,
        overflow: Arc<AtomicBool>,
        progress_state: Option<TrainingState>,
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

    let progress_state = (*state).clone();
    let stdout_handle = drain_output(stdout_reader, Arc::clone(&overflow), Some(progress_state), Some(request.job_id.clone()));
    let stderr_handle = drain_output(stderr_reader, Arc::clone(&overflow), None, None);
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(MAX_RUNTIME_SECS) {
            let _ = child.kill();
            let _ = child.wait();
            if let Ok(mut jobs) = state.0.lock() { if let Some(job) = jobs.get_mut(&request.job_id) { job.status = "failed".into(); job.stderr = "QLoRA training exceeded the runtime limit".into(); } }
            let _ = state.persist();
            return Err("QLoRA training exceeded the runtime limit".into());
        }
        if overflow.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_handle.join();
            let _ = stderr_handle.join();
            if let Ok(mut jobs) = state.0.lock() { if let Some(job) = jobs.get_mut(&request.job_id) { job.status = "failed".into(); job.stderr = "Training output exceeds the size limit".into(); } }
            let _ = state.persist();
            return Err("Training output exceeds the size limit".into());
        }
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let message = format!("Training process status check failed: {error}");
                if let Ok(mut jobs) = state.0.lock() {
                    if let Some(job) = jobs.get_mut(&request.job_id) {
                        job.status = "failed".into();
                        job.stderr = message.clone();
                    }
                }
                let _ = state.persist();
                return Err(message);
            }
        };
        if let Some(status) = status {
            let stdout = match stdout_handle.join() {
                Ok(result) => match result {
                    Ok(value) => value,
                    Err(error) => {
                        if let Ok(mut jobs) = state.0.lock() {
                            if let Some(job) = jobs.get_mut(&request.job_id) {
                                job.status = "failed".into();
                                job.stderr = error.clone();
                            }
                        }
                        let _ = state.persist();
                        return Err(error);
                    }
                },
                Err(_) => {
                    let error = "Training stdout reader thread failed".to_string();
                    if let Ok(mut jobs) = state.0.lock() {
                        if let Some(job) = jobs.get_mut(&request.job_id) {
                            job.status = "failed".into();
                            job.stderr = error.clone();
                        }
                    }
                    let _ = state.persist();
                    return Err(error);
                }
            };
            let stderr = match stderr_handle.join() {
                Ok(result) => match result {
                    Ok(value) => value,
                    Err(error) => {
                        if let Ok(mut jobs) = state.0.lock() {
                            if let Some(job) = jobs.get_mut(&request.job_id) {
                                job.status = "failed".into();
                                job.stderr = error.clone();
                            }
                        }
                        let _ = state.persist();
                        return Err(error);
                    }
                },
                Err(_) => {
                    let error = "Training stderr reader thread failed".to_string();
                    if let Ok(mut jobs) = state.0.lock() {
                        if let Some(job) = jobs.get_mut(&request.job_id) {
                            job.status = "failed".into();
                            job.stderr = error.clone();
                        }
                    }
                    let _ = state.persist();
                    return Err(error);
                }
            };
            let progress = {
                let jobs = state.0.lock().map_err(|_| "Training state lock failed".to_string())?;
                jobs.get(&request.job_id).and_then(|job| job.progress.clone())
            };
            let mut artifact = None;
            let mut result_status = if status.success() { "completed" } else { "failed" };
            let mut result_stderr = String::from_utf8_lossy(&stderr).into_owned();
            if status.success() {
                match verify_training_artifact(&root, &output, &request.job_id, &dataset) {
                    Ok(value) => artifact = Some(value),
                    Err(error) => { result_status = "failed"; result_stderr = error; }
                }
            }
            let result = TrainingRunResult {
                job_id: request.job_id,
                status: result_status,
                exit_code: status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: result_stderr,
                artifact: artifact.clone(),
            };
            if let Ok(mut jobs) = state.0.lock() {
                jobs.insert(result.job_id.clone(), TrainingJobStatus { job_id: result.job_id.clone(), status: result.status.into(), exit_code: Some(result.exit_code), stdout: result.stdout.clone(), stderr: result.stderr.clone(), artifact, progress });
            }
            state.persist()?;
            return Ok(result);
        }
        thread::sleep(Duration::from_millis(100));
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn job_status(job_id: &str, status: &str, stdout: &str, stderr: &str) -> TrainingJobStatus {
        TrainingJobStatus {
            job_id: job_id.into(),
            status: status.into(),
            exit_code: None,
            stdout: stdout.into(),
            stderr: stderr.into(),
            progress: None,
            artifact: None,
        }
    }

    #[test]
    fn persists_versioned_state_and_rejects_unknown_versions() {
        let state = PersistedTrainingState {
            version: TRAINING_STATE_VERSION,
            jobs: vec![job_status("job-1", "completed", "out", "err")],
        };
        let encoded = encode_persisted_state(&state).unwrap();
        let decoded = decode_persisted_state(&encoded).unwrap();
        assert_eq!(decoded.version, TRAINING_STATE_VERSION);
        assert_eq!(decoded.jobs[0].job_id, "job-1");

        let unsupported = serde_json::json!({"version": TRAINING_STATE_VERSION + 1, "jobs": []});
        assert!(decode_persisted_state(&serde_json::to_vec(&unsupported).unwrap()).is_err());
    }

    #[test]
    fn persisted_state_truncates_output_and_job_count() {
        let mut jobs = std::collections::HashMap::new();
        for index in 0..(MAX_PERSISTED_JOBS + 8) {
            jobs.insert(
                format!("job-{index:03}"),
                job_status(
                    &format!("job-{index:03}"),
                    "completed",
                    &"o".repeat(MAX_PERSISTED_OUTPUT_BYTES + 32),
                    &"e".repeat(MAX_PERSISTED_OUTPUT_BYTES + 32),
                ),
            );
        }
        let mut values = jobs.values().cloned().collect::<Vec<_>>();
        values.sort_by(|a, b| a.job_id.cmp(&b.job_id));
        values.truncate(MAX_PERSISTED_JOBS);
        for job in &mut values {
            if job.stdout.len() > MAX_PERSISTED_OUTPUT_BYTES {
                job.stdout.truncate(MAX_PERSISTED_OUTPUT_BYTES);
            }
            if job.stderr.len() > MAX_PERSISTED_OUTPUT_BYTES {
                job.stderr.truncate(MAX_PERSISTED_OUTPUT_BYTES);
            }
        }
        assert_eq!(values.len(), MAX_PERSISTED_JOBS);
        assert!(values.iter().all(|job| job.stdout.len() <= MAX_PERSISTED_OUTPUT_BYTES));
        assert!(values.iter().all(|job| job.stderr.len() <= MAX_PERSISTED_OUTPUT_BYTES));
    }

    #[test]
    fn malformed_persisted_state_is_rejected() {
        assert!(decode_persisted_state(b"{not-json}").is_err());
        assert!(decode_persisted_state(br#"{"version":1,"jobs":"not-an-array"}"#).is_err());
    }

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

    fn write_dataset(path: &Path) -> String {
        fs::write(path, b"{\"text\":\"hello\"}\n").unwrap();
        let (hash, _) = sha256_file(path, MAX_ARTIFACT_BYTES).unwrap();
        hash
    }

    fn write_plan(output: &Path, job_id: &str, dataset_hash: &str) {
        fs::create_dir_all(output).unwrap();
        let plan = serde_json::json!({
            "schemaVersion": TRAINING_PLAN_SCHEMA_VERSION,
            "jobId": job_id,
            "datasetSha256": dataset_hash
        });
        fs::write(output.join("training-plan.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
    }

    #[test]
    fn verifies_valid_training_artifact() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let output = root.join("output");
        fs::create_dir_all(&root).unwrap();
        let hash = write_dataset(&dataset);
        write_plan(&output, "job-1", &hash);
        let artifact = verify_training_artifact(&root, &output, "job-1", &dataset).unwrap();
        assert_eq!(artifact.schema_version, 1);
        assert_eq!(artifact.path, "output/training-plan.json");
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_missing_artifact() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let output = root.join("output");
        fs::create_dir_all(&root).unwrap();
        let hash = write_dataset(&dataset);
        assert!(verify_training_artifact(&root, &output, "job-1", &hash).is_err());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_malformed_json() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-json-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let output = root.join("output");
        fs::create_dir_all(&root).unwrap();
        let hash = write_dataset(&dataset);
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("training-plan.json"), b"not-json").unwrap();
        assert!(verify_training_artifact(&root, &output, "job-1", &dataset).is_err());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_wrong_schema_and_job_id() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-schema-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let output = root.join("output");
        fs::create_dir_all(&root).unwrap();
        let hash = write_dataset(&dataset);
        write_plan(&output, "other-job", &hash);
        assert!(verify_training_artifact(&root, &output, "job-1", &dataset).is_err());
        let plan = serde_json::json!({"schemaVersion": 999, "jobId": "job-1", "datasetSha256": hash});
        fs::write(output.join("training-plan.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
        assert!(verify_training_artifact(&root, &output, "job-1", &dataset).is_err());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_dataset_hash_mismatch() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-hash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let output = root.join("output");
        fs::create_dir_all(&root).unwrap();
        write_dataset(&dataset);
        write_plan(&output, "job-1", "deadbeef");
        assert!(verify_training_artifact(&root, &output, "job-1", &dataset).is_err());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_artifact_outside_training_root() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-path-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let outside = temp.join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let hash = write_dataset(&dataset);
        write_plan(&outside, "job-1", &hash);
        assert!(verify_training_artifact(&root, &outside, "job-1", &dataset).is_err());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_oversized_artifact() {
        let temp = std::env::temp_dir().join(format!("woho-artifact-size-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("training");
        let dataset = root.join("dataset.jsonl");
        let output = root.join("output");
        fs::create_dir_all(&root).unwrap();
        let hash = write_dataset(&dataset);
        fs::create_dir_all(&output).unwrap();
        let mut file = fs::File::create(output.join("training-plan.json")).unwrap();
        use std::io::Write;
        file.write_all(&vec![b'x'; (MAX_ARTIFACT_BYTES as usize) + 1]).unwrap();
        drop(file);
        assert!(verify_training_artifact(&root, &output, "job-1", &dataset).is_err());
        let _ = fs::remove_dir_all(temp);
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
    drop(jobs);
    state.persist()?;
    Ok(())
}
