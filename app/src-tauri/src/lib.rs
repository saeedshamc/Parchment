use mdr_core::api::{self, RenderRequest, RenderResponse};

/// Names of the bundled templates.
#[tauri::command]
fn templates() -> Vec<String> {
    api::template_names()
}

/// Renders Markdown with the given options. Pure computation: the UI does all
/// file access through the dialog/fs plugins.
#[tauri::command]
fn render(req: RenderRequest) -> Result<RenderResponse, String> {
    api::render_request(&req)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![templates, render])
        .run(tauri::generate_context!())
        .expect("error while running Parchment");
}
