#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod graph;

use graph::ParametricPlotApp;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

// WASM用のエントリーポイント
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(canvas_id: &str) -> Result<(), JsValue> {
    // パニック時のエラーハンドリングを設定
    console_error_panic_hook::set_once();

    // Get the canvas element
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    
    let web_options = eframe::WebOptions::default();
    
    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(ParametricPlotApp::default()))),
        )
        .await?;
    
    Ok(())
}

// ネイティブ用のエントリーポイントはmain.rsに残す