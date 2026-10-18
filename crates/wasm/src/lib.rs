//! Browser API. Runs inside a Web Worker; the artwork never leaves the device.

use serde::Serialize;
use smallpint_core::{analyze as core_analyze, context, Options};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

fn to_js<T: Serialize>(v: &T) -> Result<JsValue, JsError> {
    // maps as plain objects, so the UI gets ordinary JSON
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    v.serialize(&ser).map_err(|e| JsError::new(&e.to_string()))
}

/// Analyse artwork bytes. `options` is optional; see `smallpint_core::Options`.
#[wasm_bindgen]
pub fn analyze(bytes: &[u8], options: JsValue) -> Result<JsValue, JsError> {
    let opts: Options = if options.is_undefined() || options.is_null() {
        Options::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?
    };
    let report = core_analyze(bytes, &opts).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&report)
}

/// All known formats, for the "Change" picker.
#[wasm_bindgen]
pub fn formats() -> Result<JsValue, JsError> {
    to_js(&context::list_formats())
}

#[wasm_bindgen]
pub fn version() -> String {
    smallpint_core::VERSION.to_string()
}
