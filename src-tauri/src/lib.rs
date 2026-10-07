use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{path::BaseDirectory, Manager};

mod training;

const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_MODEL_BYTES: u64 = 512 * 1024 * 1024;
const MODEL_MANIFEST: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/models/manifest.json"));

#[derive(Debug, Serialize)]
struct RuntimeHealth {
    app: &'static str,
    version: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentRequest {
    #[serde(rename = "conversationId")]
    conversation_id: String,
    message: String,
    #[serde(rename = "modelId")]
    model_id: String,
    runtime: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentResponse {
    text: String,
    #[serde(rename = "modelId")]
    model_id: String,
    runtime: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ModelManifest {
    version: u32,
    models: Vec<TrustedModel>,
}

#[derive(Debug, Deserialize, Clone)]
struct TrustedModel {
    id: String,
    name: String,
    format: String,
    context_tokens: u64,
    filename: String,
    size_bytes: u64,
    sha256: String,
    download_url: String,
    source: String,
    license: String,
}

#[derive(Debug, Serialize, Clone)]
struct ManagedModel {
    id: String,
    name: String,
    format: String,
    #[serde(rename = "contextTokens")]
    context_tokens: u64,
    filename: String,
    #[serde(rename = "sizeBytes")]
    size_bytes: u64,
    source: String,
    license: String,
    installed: bool,
    verified: bool,
}

fn manifest() -> Result<ModelManifest, String> {
    let manifest = serde_json::from_str::<ModelManifest>(MODEL_MANIFEST)
        .map_err(|error| format!("Invalid bundled model manifest: {error}"))?;
    if manifest.version != 1 {
        return Err("Unsupported model manifest version".into());
    }
    if manifest.models.is_empty() {
        return Err("Model manifest is empty".into());
    }
    Ok(manifest)
}

fn trusted_model(model_id: &str) -> Result<TrustedModel, String> {
    let manifest = manifest()?;
    manifest
        .models
        .into_iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| "Unknown model".into())
}

fn trusted_download_host(host: &str) -> bool {
    host == "huggingface.co" || host.ends_with(".huggingface.co") || host.ends_with(".xethub.hf.co")
}

fn validate_model(model: &TrustedModel) -> Result<(), String> {
    if !model.id.bytes().enumerate().all(|(index, byte)| {
        (byte.is_ascii_lowercase() || byte.is_ascii_digit())
            || (index > 0 && matches!(byte, b'.' | b'_' | b'-'))
    }) {
        return Err("Invalid trusted model id".into());
    }
    if model.format != "gguf" || model.filename != format!("{}.gguf", model.id) {
        return Err("Invalid trusted model filename or format".into());
    }
    if model.size_bytes == 0 || model.size_bytes > MAX_MODEL_BYTES {
        return Err("Trusted model exceeds the supported size limit".into());
    }
    if !model
        .sha256
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit())
        || model.sha256.len() != 64
    {
        return Err("Invalid trusted model SHA-256".into());
    }
    let url = reqwest::Url::parse(&model.download_url)
        .map_err(|_| "Invalid trusted model URL".to_string())?;
    if url.scheme() != "https"
        || !url
            .host_str()
            .map(trusted_download_host)
            .unwrap_or(false)
    {
        return Err("Trusted model source is not allowed".into());
    }
    Ok(())
}

fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?
        .join("models"))
}

fn model_path(app: &tauri::AppHandle, model: &TrustedModel) -> Result<PathBuf, String> {
    validate_model(model)?;
    Ok(models_dir(app)?.join(&model.filename))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("Unable to open model: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Unable to read model: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_model_file(app: &tauri::AppHandle, model: &TrustedModel) -> Result<bool, String> {
    let path = model_path(app, model)?;
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Unable to inspect model: {error}")),
    };
    if metadata.len() != model.size_bytes {
        return Ok(false);
    }
    Ok(sha256_file(&path)? == model.sha256)
}

fn managed_model(
    app: &tauri::AppHandle,
    model: TrustedModel,
    verified: bool,
) -> Result<ManagedModel, String> {
    let path = model_path(app, &model)?;
    let installed = path.is_file();
    Ok(ManagedModel {
        id: model.id,
        name: model.name,
        format: model.format,
        context_tokens: model.context_tokens,
        filename: model.filename,
        size_bytes: model.size_bytes,
        source: model.source,
        license: model.license,
        installed,
        verified: installed && verified,
    })
}

fn validate_agent_request(request: &AgentRequest) -> Result<(), String> {
    if request.runtime != "local" {
        return Err("Only local runtime is currently supported".into());
    }
    if request.conversation_id.is_empty()
        || request.conversation_id.len() > 256
        || request.message.is_empty()
        || request.message.len() > MAX_INPUT_BYTES
        || request.model_id.is_empty()
        || request.model_id.len() > 128
    {
        return Err("Agent request exceeds validation limits".into());
    }
    if !request.model_id.bytes().enumerate().all(|(index, byte)| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || (index > 0 && matches!(byte, b'.' | b'_' | b'-'))
    }) {
        return Err("Invalid model id".into());
    }
    Ok(())
}

fn inside(root: &Path, candidate: &Path) -> bool {
    let root = match root.canonicalize() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let candidate = match candidate.canonicalize() {
        Ok(value) => value,
        Err(_) => return false,
    };
    candidate.starts_with(root)
}

fn worker_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        if let Ok(path) = std::env::var("WOHO_AGENT_BRIDGE") {
            return Ok(PathBuf::from(path));
        }
    }
    let packaged = if cfg!(windows) {
        app.path()
            .resolve("agent/woho-agent.exe", BaseDirectory::Resource)
            .ok()
    } else {
        app.path()
            .resolve("agent/woho-agent", BaseDirectory::Resource)
            .ok()
    };
    if let Some(path) = packaged {
        if path.is_file() {
            return Ok(path);
        }
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/agent/agent-bridge.mjs"))
}

fn llama_runtime_dir(app: &tauri::AppHandle) -> Result<(PathBuf, bool), String> {
    if cfg!(debug_assertions) {
        if let Ok(path) = std::env::var("WOHO_LLAMA_RUNTIME_DIR") {
            return Ok((PathBuf::from(path), false));
        }
    }
    if let Ok(path) = app.path().resolve("llama", BaseDirectory::Resource) {
        if path.is_dir() {
            return Ok((path, true));
        }
    }
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;
    Ok((app_data.join("runtimes").join("llama"), false))
}

fn run_agent_bridge(app: &tauri::AppHandle, request: AgentRequest) -> Result<AgentResponse, String> {
    validate_agent_request(&request)?;
    let model = trusted_model(&request.model_id)?;
    if !verify_model_file(app, &model)? {
        return Err("Model is not installed or failed SHA-256 verification".into());
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;
    let (runtime_dir, packaged_runtime) = llama_runtime_dir(app)?;
    let model_dir = app_data.join("models");
    let executable = if cfg!(windows) {
        runtime_dir.join("llama-cli.exe")
    } else {
        runtime_dir.join("llama-cli")
    };
    let worker = worker_path(app)?;

    if !worker.is_file() {
        return Err("Agent bridge sidecar is not installed".into());
    }
    if !executable.is_file() || !inside(&runtime_dir, &executable) {
        return Err("llama.cpp runtime is not installed in the managed runtime directory".into());
    }
    if packaged_runtime && std::env::var("WOHO_LLAMA_EXECUTABLE").is_ok() {
        return Err("Packaged runtime cannot be overridden by environment configuration".into());
    }
    if !model_dir.is_dir() {
        return Err("Model directory is not installed".into());
    }

    let payload = serde_json::to_vec(&request)
        .map_err(|error| format!("Failed to encode agent request: {error}"))?;

    let packaged_agent = cfg!(windows)
        && worker
            .extension()
            .map(|extension| extension.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);

    let mut command = if packaged_agent {
        let mut command = Command::new(&worker);
        command
            .env_remove("WOHO_AGENT_NODE")
            .env_remove("WOHO_AGENT_BRIDGE");
        command
    } else {
        let node = if cfg!(debug_assertions) {
            std::env::var("WOHO_AGENT_NODE").unwrap_or_else(|_| "node".to_string())
        } else {
            "node".to_string()
        };
        let mut command = Command::new(node);
        command.arg(&worker);
        command
    };

    let mut child = command
        .env("WOHO_LLAMA_RUNTIME_DIR", &runtime_dir)
        .env("WOHO_MODEL_DIR", &model_dir)
        .env("WOHO_LLAMA_EXECUTABLE", &executable)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Failed to start agent bridge: {error}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("Agent bridge stdin unavailable")?;
        stdin
            .write_all(&payload)
            .map_err(|error| format!("Failed to send agent request: {error}"))?;
        stdin
            .write_all(b"\n")
            .map_err(|error| format!("Failed to terminate agent request: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Agent bridge process failed: {error}"))?;

    if output.stdout.len() > MAX_OUTPUT_BYTES {
        return Err("Agent bridge response exceeds size limit".into());
    }

    let line = String::from_utf8(output.stdout)
        .map_err(|_| "Agent bridge returned invalid UTF-8".to_string())?;
    let envelope: serde_json::Value = serde_json::from_str(line.trim())
        .map_err(|_| "Agent bridge returned invalid JSON".to_string())?;

    if envelope.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(envelope
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Agent bridge failed")
            .to_string());
    }

    serde_json::from_value(
        envelope
            .get("result")
            .cloned()
            .ok_or("Agent bridge returned no result")?,
    )
    .map_err(|_| "Agent bridge returned an invalid result".to_string())
}

#[tauri::command]
fn health() -> RuntimeHealth {
    RuntimeHealth {
        app: "WoHo AI Desktop",
        version: env!("CARGO_PKG_VERSION"),
        status: "ok",
    }
}

#[tauri::command]
fn runtime_status() -> &'static str {
    "ready"
}

#[tauri::command]
fn permission_check(capability: String) -> bool {
    matches!(
        capability.as_str(),
        "read_project" | "write_project" | "run_tool"
    )
}

#[tauri::command]
fn model_list(app: tauri::AppHandle) -> Result<Vec<ManagedModel>, String> {
    manifest()?
        .models
        .into_iter()
        .map(|model| {
            let verified = verify_model_file(&app, &model)?;
            managed_model(&app, model, verified)
        })
        .collect()
}

#[tauri::command]
async fn model_install(app: tauri::AppHandle, model_id: String) -> Result<ManagedModel, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let model = trusted_model(&model_id)?;
        validate_model(&model)?;
        let models_dir = models_dir(&app)?;
        fs::create_dir_all(&models_dir)
            .map_err(|error| format!("Unable to create model directory: {error}"))?;

        let target = model_path(&app, &model)?;
        if target.is_file() {
            if verify_model_file(&app, &model)? {
                return managed_model(&app, model, true);
            }
            return Err("A model file already exists but failed verification; remove it before reinstalling".into());
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "System clock is invalid".to_string())?
            .as_nanos();
        let temp = models_dir.join(format!(".{}.{}.download", model.id, stamp));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error| format!("Unable to create temporary model file: {error}"))?;

        let result = (|| {
            let response = reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    if attempt.previous().len() >= 5 {
                        attempt.error("too many model download redirects")
                    } else if attempt.url().scheme() == "https"
                        && attempt
                            .url()
                            .host_str()
                            .map(trusted_download_host)
                            .unwrap_or(false)
                    {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }))
                .build()
                .map_err(|error| format!("Unable to create HTTPS client: {error}"))?
                .get(&model.download_url)
                .header(reqwest::header::ACCEPT, "application/octet-stream")
                .send()
                .map_err(|error| format!("Model download failed: {error}"))?;

            if !response.status().is_success() {
                return Err(format!("Model download returned HTTP {}", response.status()));
            }
            if let Some(length) = response.content_length() {
                if length != model.size_bytes || length > MAX_MODEL_BYTES {
                    return Err("Downloaded model size does not match the trusted manifest".into());
                }
            }

            let mut response = response;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 1024 * 1024];
            let mut total = 0u64;
            loop {
                let read = response
                    .read(&mut buffer)
                    .map_err(|error| format!("Model download read failed: {error}"))?;
                if read == 0 {
                    break;
                }
                total = total
                    .checked_add(read as u64)
                    .ok_or("Model size overflow")?;
                if total > model.size_bytes || total > MAX_MODEL_BYTES {
                    return Err("Downloaded model exceeds the trusted size limit".into());
                }
                hasher.update(&buffer[..read]);
                file.write_all(&buffer[..read])
                    .map_err(|error| format!("Model write failed: {error}"))?;
            }
            file.sync_all()
                .map_err(|error| format!("Model flush failed: {error}"))?;

            if total != model.size_bytes {
                return Err("Downloaded model size does not match the trusted manifest".into());
            }
            let digest = format!("{:x}", hasher.finalize());
            if digest != model.sha256 {
                return Err("Downloaded model SHA-256 does not match the trusted manifest".into());
            }
            fs::rename(&temp, &target)
                .map_err(|error| format!("Unable to atomically install model: {error}"))?;
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result?;
        managed_model(&app, model, true)
    })
    .await
    .map_err(|error| format!("Model install task failed: {error}"))?
}

#[tauri::command]
fn model_verify(app: tauri::AppHandle, model_id: String) -> Result<ManagedModel, String> {
    let model = trusted_model(&model_id)?;
    let verified = verify_model_file(&app, &model)?;
    managed_model(&app, model, verified)
}

#[tauri::command]
fn model_remove(app: tauri::AppHandle, model_id: String) -> Result<(), String> {
    let model = trusted_model(&model_id)?;
    let path = model_path(&app, &model)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Unable to remove model: {error}")),
    }
}

#[tauri::command]
async fn agent_chat(
    app: tauri::AppHandle,
    request: AgentRequest,
) -> Result<AgentResponse, String> {
    tauri::async_runtime::spawn_blocking(move || run_agent_bridge(&app, request))
        .await
        .map_err(|error| format!("Agent bridge task failed: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            health,
            runtime_status,
            permission_check,
            model_list,
            model_install,
            model_verify,
            model_remove,
            agent_chat,
            training::run_training
        ])
        .run(tauri::generate_context!())
        .expect("error while running WoHo AI Desktop");
}
