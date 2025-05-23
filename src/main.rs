use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints};
use rquickjs::{Runtime, Context as JsContext, Result as JsResult};
use rquickjs::function::Func;

use std::cell::RefCell;
use std::rc::Rc;

// スライダ情報を保持する構造体
#[derive(Clone)]
struct SliderParam {
    name: String,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
}

// アプリケーションの状態を保持する構造体
struct ParametricPlotApp {
    sliders: Vec<SliderParam>,
    js_context: JsContext,
    js_code_evaluated: bool,
    graph_lines: Rc<RefCell<Vec<(String, Vec<[f64; 2]>)>>>, // グラフデータを保持
}

impl Default for ParametricPlotApp {
    fn default() -> Self {
        let runtime = Runtime::new().unwrap();
        let js_context = JsContext::full(&runtime).unwrap();
        Self {
            sliders: Vec::new(),
            js_context,
            js_code_evaluated: false,
            graph_lines: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl ParametricPlotApp {}

impl App for ParametricPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut js_error: Option<String> = None;
            let mut need_redraw = false;

            // グラフエリアのサイズを画面全体に設定
            let available_size = ui.available_size();

            // --- プロット領域の作成（背景として配置）---
            let plot = Plot::new("parametric_plot")
                .show_background(true)
                .show_axes([true, true])
                .min_size(available_size)
                .width(available_size.x)
                .height(available_size.y)
                .data_aspect(1.0)
                .x_axis_label("x")
                .y_axis_label("y");

            // プロット描画
            plot.show(ui, |plot_ui| {
                for (name, points) in self.graph_lines.borrow().iter() {
                    let line = Line::new(name, PlotPoints::new(points.clone()))
                        .color(egui::Color32::from_rgb(200, 100, 0))
                        .name(name);
                    plot_ui.line(line);
                }
            });

            // --- スライダーを重ねて表示 ---
            if !self.sliders.is_empty() {
                // 左上にスライダーパネルを配置
                egui::Window::new("パラメータ")
                    .fixed_pos(egui::pos2(10.0, 10.0))
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.set_min_width(100.0);
                        for slider in &mut self.sliders {
                            if ui.add(egui::Slider::new(&mut slider.value, slider.min..=slider.max)
                                .text(&slider.name)
                                .step_by(slider.step)).changed() 
                            {
                                need_redraw = true;
                            }
                        }
                    });
            }

            // --- JavaScript関連の処理 ---
            self.js_context.with(|js_ctx| {
                // 最初の実行時にのみJavaScript関数を定義
                if !self.js_code_evaluated {
                    let sliders_rc = Rc::new(RefCell::new(Vec::new()));
                    let sliders_api = sliders_rc.clone();
                    let graph_lines_api = self.graph_lines.clone();

                    // addSlider API
                    let add_slider = Func::from(move |name: String, min: f64, max: f64, step: f64, default: f64| {
                        sliders_api.borrow_mut().push(SliderParam {
                            name: name.clone(), min, max, step, value: default
                        });
                    });
                    js_ctx.globals().set("addSlider", add_slider).unwrap();

                    // addParametricGraph API
                    let add_parametric_graph = Func::from(move |f: rquickjs::Function, range: rquickjs::Object, name: String| -> JsResult<()> {
                        let min: f64 = range.get("min").unwrap_or(0.0);
                        let max: f64 = range.get("max").unwrap_or(2.0 * std::f64::consts::PI);
                        let delta: Option<f64> = range.get("delta").ok();
                        let mut points = Vec::new();
                        if let Some(delta) = delta {
                            let mut t = min;
                            while t <= max {
                                let xy: Vec<f64> = f.call((t,))?;
                                if xy.len() == 2 {
                                    points.push([xy[0], xy[1]]);
                                }
                                t += delta;
                            }
                        } else {
                            let num_points: usize = range.get("num_points").unwrap_or(500);
                            for i in 0..num_points {
                                let t = min + (i as f64 / (num_points - 1) as f64) * (max - min);
                                let xy: Vec<f64> = f.call((t,))?;
                                if xy.len() == 2 {
                                    points.push([xy[0], xy[1]]);
                                }
                            }
                        }
                        graph_lines_api.borrow_mut().push((name, points));
                        Ok(())
                    });
                    js_ctx.globals().set("addParametricGraph", add_parametric_graph).unwrap();

                    // JSコードの評価
                    let js_code = r#"
                        // 円と薔薇曲線を描画する関数を定義
                        function setup() {
                            // パラメータをスライダーで定義
                            addSlider("a", 0.1, 2.0, 0.1, 1.0);  // 円の横サイズ
                            addSlider("b", 0.1, 2.0, 0.1, 1.0);  // 円の縦サイズ
                            addSlider("k", 1, 20, 1, 9);         // 薔薇曲線のローブ数
                            addSlider("r", 0.1, 2.0, 0.1, 1.0);  // 薔薇曲線の大きさ
                        }
                        function draw() {
                            // 円 (a,bで縦横比を制御)
                            addParametricGraph(
                                function(t) { return [a * Math.cos(t), b * Math.sin(t)]; },
                                { min: 0, max: 2 * Math.PI, num_points: 1000 },
                                `楕円 (a=${a.toFixed(1)}, b=${b.toFixed(1)})`
                            );
                            // 薔薇曲線 (k=ローブ数, r=サイズ)
                            addParametricGraph(
                                function(t) {
                                    let radius = r * Math.cos(k * t);
                                    return [radius * Math.cos(t), radius * Math.sin(t)];
                                },
                                { min: 0, max: 2 * Math.PI, num_points: 1000 },
                                `薔薇曲線 (k=${k}, r=${r.toFixed(1)})`
                            );
                        }
                    "#;

                    if let Err(e) = js_ctx.eval::<(), _>(js_code) {
                        js_error = Some(format!("JavaScript parse error: {e:?}"));
                        return;
                    }

                    // スライダーの初期化
                    let setup_exists = js_ctx.eval::<bool, _>("typeof setup === 'function'").unwrap_or(false);
                    if setup_exists {
                        if let Err(e) = js_ctx.eval::<(), _>("setup();") {
                            js_error = Some(format!("setup() error: {e:?}"));
                            return;
                        }
                        self.sliders = sliders_rc.borrow().clone();
                        need_redraw = true;
                    } else {
                        js_error = Some("setup関数が定義されていません".to_string());
                        return;
                    }

                    self.js_code_evaluated = true;
                }

                // 初期表示時またはスライダー変更時に再描画
                if need_redraw || self.graph_lines.borrow().is_empty() {
                    // スライダー値をJSグローバルに注入
                    for slider in &self.sliders {
                        js_ctx.globals().set(slider.name.as_str(), slider.value).unwrap();
                    }

                    // グラフデータをクリア
                    self.graph_lines.borrow_mut().clear();

                    // draw関数実行
                    if let Err(e) = js_ctx.eval::<(), _>("draw();") {
                        js_error = Some(format!("draw() error: {}", e));
                        return;
                    }
                }
            });

            // エラー表示（もしあれば）
            if let Some(err) = js_error {
                egui::Window::new("エラー")
                    .fixed_pos(egui::pos2(10.0, ui.available_rect_before_wrap().bottom() - 40.0))
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.colored_label(egui::Color32::RED, format!("JSエラー: {}", err));
                    });
                println!("JSエラー: {}", err);
            }
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
