use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints};
use rquickjs::{Runtime, Context as JsContext, Result as JsResult};
use rquickjs::function::Func;

// アプリケーションの状態を保持する構造体
struct ParametricPlotApp;

impl Default for ParametricPlotApp {
    fn default() -> Self {
        Self
    }
}

impl App for ParametricPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // --- Rust→JS API: addParametricGraph ---
            let rt = Runtime::new().unwrap();
            let js_ctx = JsContext::full(&rt).unwrap();
            let mut graph_lines: Vec<(String, Vec<[f64; 2]> )> = Vec::new();
            js_ctx.with(|js_ctx| {
                use std::rc::Rc;
                use std::cell::RefCell;
                let graph_lines_rc = Rc::new(RefCell::new(Vec::new()));
                let graph_lines_api = graph_lines_rc.clone();
                let add_parametric_graph = Func::from(move |f: rquickjs::Function, range: rquickjs::Object, name: String| {
                    let min: f64 = range.get("min").unwrap_or(0.0);
                    let max: f64 = range.get("max").unwrap_or(2.0 * std::f64::consts::PI);
                    let delta: Option<f64> = range.get("delta").ok();
                    let mut points = Vec::new();
                    if let Some(delta) = delta {
                        let mut t = min;
                        while t <= max {
                            let xy: Vec<f64> = f.call((t,)).unwrap();
                            if xy.len() == 2 {
                                points.push([xy[0], xy[1]]);
                            }
                            t += delta;
                        }
                    } else {
                        let num_points: usize = range.get("num_points").unwrap_or(500);
                        for i in 0..num_points {
                            let t = min + (i as f64 / (num_points - 1) as f64) * (max - min);
                            let xy: Vec<f64> = f.call((t,)).unwrap();
                            if xy.len() == 2 {
                                points.push([xy[0], xy[1]]);
                            }
                        }
                    }
                    graph_lines_api.borrow_mut().push((name.clone(), points));
                });
                js_ctx.globals().set("addParametricGraph", add_parametric_graph).unwrap();
                // サンプル: JSでパラメータも指定
                let js_code = r#"
                    let a = 1.0;
                    let b = 1.0;
                    // 円
                    addParametricGraph(
                        function(t) { return [a * Math.cos(t), b * Math.sin(t)]; },
                        { min: 0, max: 2 * Math.PI, num_points: 1000 },
                        "媒介変数曲線"
                    );
                    // 薔薇曲線（ローズカーブ）: r = cos(kθ)
                    let k = 9;
                    let r = 1.0;
                    addParametricGraph(
                        function(t) {
                            let radius = r * Math.cos(k * t);
                            return [radius * Math.cos(t), radius * Math.sin(t)];
                        },
                        { min: 0, max: 2 * Math.PI, num_points: 1000 },
                        `薔薇曲線 k=${k}`
                    );
                "#;
                js_ctx.eval::<(), _>(js_code).unwrap();
                let graph_lines_final = match Rc::try_unwrap(graph_lines_rc) {
                    Ok(rc) => rc.into_inner(),
                    Err(rc) => rc.borrow().clone(),
                };
                graph_lines = graph_lines_final;
            });
            // --- グラフ描画 ---
            let window_height = ctx.input(|input| input.screen_rect().height());
            let window_width = ctx.input(|input| input.screen_rect().width());
            let plot_size = egui::vec2(window_width, window_height);
            let plot = Plot::new("parametric_plot")
                .show_background(true)
                .show_axes([true, true])
                .min_size(plot_size)
                .width(plot_size.x)
                .height(plot_size.y)
                .data_aspect(1.0)
                .x_axis_label("x")
                .y_axis_label("y");
            plot.show(ui, |plot_ui| {
                for (name, points) in &graph_lines {
                    let line = Line::new(name.as_str(), PlotPoints::new(points.clone()))
                        .color(egui::Color32::from_rgb(200, 100, 0))
                        .name(name);
                    plot_ui.line(line);
                }
            });
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
