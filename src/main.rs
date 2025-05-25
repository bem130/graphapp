#![cfg(not(target_arch = "wasm32"))]

mod graph;

use eframe::egui;
use graph::{ParametricPlotApp, setup_logging};

fn main() -> eframe::Result<()> {
    // ログ設定を初期化
    setup_logging();

    // アプリケーションオプションの設定
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Egui Parametric Plot"),
        ..Default::default()
    };

    eframe::run_native(
        "Egui Parametric Plot",
        native_options,
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
            Ok(Box::new(ParametricPlotApp::default()))
        }),
    )
}