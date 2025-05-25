#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod app;

use eframe::egui;

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
            Box::new(|cc| {
                // フォント定義をカスタマイズ
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "NotoSerifJP".to_owned(),
                    egui::FontData::from_static(include_bytes!("fonts/NotoSerifJP-VariableFont_wght.ttf")).into(),
                );
                fonts.font_data.insert(
                    "MPLUS1Code".to_owned(),
                    egui::FontData::from_static(include_bytes!("fonts/MPLUS1Code-VariableFont_wght.ttf")).into(),
                );
                if let Some(mono) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    mono.insert(0, "MPLUS1Code".to_owned());
                }
                if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    prop.insert(0, "NotoSerifJP".to_owned());
                }
                cc.egui_ctx.set_fonts(fonts);
                Ok(Box::new(app::MyApp::default()))
            }),
        )
        .await?;
    
    Ok(())
}

// ネイティブ用のエントリーポイントはmain.rsに残す