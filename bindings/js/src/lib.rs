use wasm_bindgen::prelude::*;
use js_sys::Array;

#[wasm_bindgen]
pub struct Converter {
    inner: norg_rs::export::Exporter,
}

#[wasm_bindgen]
impl Converter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }
    // #[wasm_bindgen]
    // pub fn run_janet(&self, code: &str) {
    //     let _ = self.inner.run_janet(code);
    // }
    #[wasm_bindgen]
    pub fn convert(&mut self, ast: JsValue) -> Result<JsValue, JsValue> {
        // let ast = norg_rs::parser::parse(document.as_bytes());
        let ast = serde_wasm_bindgen::from_value(ast)?;
        let (html, ctx) = self.inner.export(norg_rs::export::ExportTarget::Html, ast, None)
            .map_err(|e| JsValue::from_str(&format!("Export failed: {:?}", e)))?;
        let ctx_js = serde_wasm_bindgen::to_value(&ctx)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize export context: {:?}", e)))?;
        let res = Array::new();
        res.push(&JsValue::from_str(&html));
        res.push(&ctx_js);
        Ok(res.into())
    }
}
