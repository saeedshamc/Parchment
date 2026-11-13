//! WebAssembly bindings. Everything goes through JSON strings so the JS side
//! stays tiny: `render(requestJson) -> responseJson`.

use wasm_bindgen::prelude::*;

/// Names of the bundled templates.
#[wasm_bindgen]
pub fn templates() -> Vec<String> {
    mdr_core::api::template_names()
}

/// `request_json` follows `mdr_core::api::RenderRequest`; the return value is
/// `RenderResponse` as JSON. Errors are thrown as JS exceptions (strings).
#[wasm_bindgen]
pub fn render(request_json: &str) -> Result<String, JsValue> {
    mdr_core::api::render_json(request_json).map_err(|e| JsValue::from_str(&e))
}
