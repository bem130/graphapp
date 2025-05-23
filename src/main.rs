use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints};
use rquickjs::{Runtime, Context as JsContext, Result as JsResult};
use rquickjs::function::Func;

// アプリケーションの状態を保持する構造体
struct ParametricPlotApp {
    a: f64, // x座標の振幅
    b: f64, // y座標の振幅
    t_min: f64, // パラメータtの最小値
    t_max: f64, // パラメータtの最大値
    num_points: usize, // プロットする点の数
}

impl Default for ParametricPlotApp {
    fn default() -> Self {
        Self {
            a: 1.0,
            b: 1.0,
            t_min: 0.0,
            t_max: 2.0 * std::f64::consts::PI, // 0から2πまで
            num_points: 50, // デフォルトで500点
        }
    }
}

impl App for ParametricPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // --- JavaScriptで点列を生成 ---
            let rt = Runtime::new().unwrap();
            let js_ctx = JsContext::full(&rt).unwrap();
            let mut points: Vec<[f64; 2]> = Vec::new();
            js_ctx.with(|js_ctx| {
                // RustのパラメータをJSに渡す
                js_ctx.globals().set("a", self.a).unwrap();
                js_ctx.globals().set("b", self.b).unwrap();
                js_ctx.globals().set("t_min", self.t_min).unwrap();
                js_ctx.globals().set("t_max", self.t_max).unwrap();
                js_ctx.globals().set("num_points", self.num_points as i32).unwrap();
                // JSで点列を生成
                let js_code = r#"
                    let points = [];
                    let t_range = t_max - t_min;
                    for (let i = 0; i < num_points; ++i) {
                        let t = t_min + (i / (num_points - 1)) * t_range;
                        let x = a * Math.cos(t);
                        let y = b * Math.sin(t);
                        points.push([x, y]);
                    }
                    points;
                "#;
                let js_points: Vec<Vec<f64>> = js_ctx.eval(js_code).unwrap();
                points = js_points.into_iter().map(|xy| [xy[0], xy[1]]).collect();
            });
            // ウインドウサイズを取得
            let window_height = ctx.input(|input| input.screen_rect().height());
            let window_width = ctx.input(|input| input.screen_rect().width());
            let plot_size = egui::vec2(window_width, window_height);
            let line = Line::new("媒介変数曲線", PlotPoints::new(points))
                .color(egui::Color32::from_rgb(200, 100, 0))
                .name("媒介変数曲線");
            Plot::new("parametric_plot")
                .show_background(true)
                .show_axes([true, true])
                .min_size(plot_size)
                .width(plot_size.x)
                .height(plot_size.y)
                .data_aspect(1.0)
                .x_axis_label("x")
                .y_axis_label("y")
                .show(ui, |plot_ui| plot_ui.line(line));
        });
    }
}

fn main() -> eframe::Result<()> {
    // アプリケーションオプションの設定
    let native_options = eframe::NativeOptions {
        // window size fields removed for compatibility
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
            // プロポーショナルファミリーの先頭にNotoSerifJPを追加
            if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                prop.insert(0, "NotoSerifJP".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(ParametricPlotApp::default()))
        }),
    )
}
