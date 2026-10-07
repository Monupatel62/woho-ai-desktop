use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

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
    if !request
        .model_id
        .bytes()
        .enumerate()
        .all(|(index, byte)| byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'.' | b'_' | b'-')
            && index > 0)
    {
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

fn worker_path() -> PathBuf {
    if let Ok(path) = std::env::var("WOHO_AGENT_BRIDGE") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/agent/agent-bridge.mjs")
}

fn run_agent_bridge(app: &tauri::AppHandle, request: AgentRequest) -> Result<AgentResponse, String> {
    validate_agent_request(&request)?;

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;
    let runtime_dir = app_data.join("runtimes").join("llama");
    let model_dir = app_data.join("models");
    let executable = if cfg!(windows) {
        runtime_dir.join("llama-cli.exe")
    } else {
        runtime_dir.join("llama-cli")
    };
    let worker = worker_path();

    if !worker.is_file() {
        return Err("Agent bridge worker is not installed. Development uses src/agent/agent-bridge.mjs; production will use the packaged sidecar.".into());
    }
    if !executable.is_file() || !inside(&runtime_dir, &executable) {
        return Err("llama.cpp runtime is not installed in the managed runtime directory".into());
    }
    if !model_dir.is_dir() {
        return Err("Model directory is not installed".into());
    }

    let node = std::env::var("WOHO_AGENT_NODE").unwrap_or_else(|_| "node".to_string());
    let payload = serde_json::to_vec(&request).map_err(|error| format!("Failed to encode agent request: {error}"))?;

    let mut child = Command::new(node)
        .arg(&worker)
        .env("WOHO_LLAMA_RUNTIME_DIR", &runtime_dir)
        .env("WOHO_MODEL_DIR", &model_dir)
        .env("WOHO_LLAMA_EXECUTABLE", &executable)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Failed to start agent bridge: {error}"))?;

    {
        let stdin = child.stdin.as_mut().ok_or("Agent bridge stdin unavailable")?;
        stdin.write_all(&payload).map_err(|error| format!("Failed to send agent request: {error}"))?;
        stdin.write_all(b"\n").map_err(|error| format!("Failed to terminate agent request: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Agent bridge process failed: {error}"))?;

    if output.stdout.len() > MAX_OUTPUT_BYTES {
        return Err("Agent bridge response exceeds size limit".into());
    }

    let line = String::from_utf8(output.stdout)
        .map_err(|_| "Agent bridge returned invalid UTF-8".to_string())?;
    let envelope: serde_json::Value =
        serde_json::from_str(line.trim()).map_err(|_| "Agent bridge returned invalid JSON".to_string())?;

    if envelope.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(envelope
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Agent bridge failed")
            .to_string());
    }

    serde_json::from_value(envelope.get("result").cloned().ok_or("Agent bridge returned no result")?)
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
    matches!(capability.as_str(), "read_project" | "write_project" | "run_tool")
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
            agent_chat
        ])
        .run(tauri::generate_context!())
        .expect("error while running WoHo AI Desktop");
}
