use serde::Serialize;

#[derive(Debug, Serialize)]
struct RuntimeHealth {
    app: &'static str,
    version: &'static str,
    status: &'static str,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![health, runtime_status, permission_check])
        .run(tauri::generate_context!())
        .expect("error while running WoHo AI Desktop");
}
