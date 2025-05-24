use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints, Arrows};
use egui::Color32;
use rquickjs::{Runtime, Context as JsContext, Result as JsResult};
use rquickjs::function::Func;
use colored::*;
use egui_extras::syntax_highlighting;

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
    graph_lines: Rc<RefCell<Vec<(String, Vec<[f64; 2]>)>>>,
    vectors: Rc<RefCell<Vec<(String, Vec<[f64; 2]>, Vec<[f64; 2]>)>>>,  // ベクトルデータを保持 (名前、始点の配列、終点の配列)
    js_code: String, // JavaScriptエディタ用
    last_js_code: String, // 前回実行したJSコード
}

impl Default for ParametricPlotApp {
    fn default() -> Self {
        let runtime = Runtime::new().unwrap();
        let js_context = JsContext::full(&runtime).unwrap();
        let default_js_code = r#"
// 円と薔薇曲線を描画する関数を定義
function setup() {
    // パラメータをスライダーで定義
    addSlider('a', { min: 0.1, max: 2.0, step: 0.1, default: 1.0 });
    addSlider('b', { min: 0.1, max: 2.0, step: 0.1, default: 1.0 });
    addSlider('k', { min: 1, max: 20, step: 1, default: 9 });
    addSlider('r', { min: 0.1, max: 2.0, step: 0.1, default: 1.0 });
    addSlider('n', { min: 0, max: 5, step: 0.01, default: 1 });
}
function draw() {
    addParametricGraph(
        `楕円 (a=${a.toFixed(1)}, b=${b.toFixed(1)})`,
        function(t) { return [a * Math.cos(t), b * Math.sin(t)]; },
        { min: 0, max: 2 * Math.PI, num_points: 1000 }
    );
    addParametricGraph(
        `薔薇曲線 (k=${k}, r=${r.toFixed(1)})`,
        function(t) {
            let radius = r * Math.cos(k * t);
            return [radius * Math.cos(t), radius * Math.sin(t)];
        },
        { min: 0, max: 2 * Math.PI, num_points: 1000 }
    );
    let t = (n) * 2 * Math.PI;
    let start = function(t) { return [a * Math.cos(t), b * Math.sin(t)]; };
    let tangent = function(t) { return [-a * Math.sin(t), b * Math.cos(t)]; };
    addVector(`接線${n}`, start, tangent, t);
}
"#.to_string();
        Self {
            sliders: Vec::new(),
            js_context,
            js_code_evaluated: false,
            graph_lines: Rc::new(RefCell::new(Vec::new())),
            vectors: Rc::new(RefCell::new(Vec::new())),
            js_code: default_js_code.clone(),
            last_js_code: default_js_code,
        }
    }
}

impl ParametricPlotApp {}

impl App for ParametricPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut js_error: Option<String> = None;
        let mut need_redraw = false;
        let mut js_code_changed = false;
        egui::SidePanel::right("js_editor_panel").min_width(350.0).show(ctx, |ui| {
            ui.heading("JavaScript エディタ");
            ui.label("グラフ描画用のJavaScriptコードを編集できます。");
            let mut theme = syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
            ui.collapsing("Theme", |ui| {
                ui.group(|ui| {
                    theme.ui(ui);
                    theme.clone().store_in_memory(ui.ctx());
                });
            });
            let mut layouter = |ui: &egui::Ui, buf: &str, wrap_width: f32| {
                let mut layout_job = syntax_highlighting::highlight(
                    ui.ctx(),
                    ui.style(),
                    &theme,
                    buf,
                    "js",
                );
                layout_job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(layout_job))
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.js_code)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(24)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                );
                if response.changed() {
                    js_code_changed = true;
                }
                if ui.button("グラフを更新").clicked() {
                    js_code_changed = true;
                }
            });
        });
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
                // 通常の曲線を描画
                for (name, points) in self.graph_lines.borrow().iter() {                    let line = Line::new(name, PlotPoints::new(points.clone()))
                        .color(Color32::from_rgb(200, 100, 0));
                    plot_ui.line(line);
                }

                // ベクトルを描画
                for (name, origins, tips) in self.vectors.borrow().iter() {                    let arrows = Arrows::new(
                        name,                              // 名前
                        PlotPoints::new(origins.clone()),  // 始点の配列
                        PlotPoints::new(tips.clone())      // 終点の配列
                    ).color(Color32::from_rgb(0, 150, 200));
                    plot_ui.arrows(arrows);
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
                // JSコードが変更された場合は再評価
                if js_code_changed || !self.js_code_evaluated || self.js_code != self.last_js_code {
                    self.js_code_evaluated = false;
                    self.last_js_code = self.js_code.clone();
                    // Rust側stdout/stderrをJSに提供
                    let stdout = Func::from(|content: String| {
                        println!("{}", content);
                    });
                    js_ctx.globals().set("stdout", stdout).unwrap();
                    let stderr = Func::from(|content: String| {
                        eprintln!("{}", content.red());
                    });
                    js_ctx.globals().set("stderr", stderr).unwrap();
                    // JS側でconsole.log/console.errorをstdout/stderr経由でJSON出力するように定義
                    let console_js = r#"
                        try {
                            if (typeof globalThis.console !== 'object' || globalThis.console === null) {
                                globalThis.console = {};
                            }
                            globalThis.console.log = function(...args) {
                                try { stdout(JSON.stringify(args)); } catch(e) {}
                            };
                            globalThis.console.error = function(...args) {
                                try { stderr(JSON.stringify(args)); } catch(e) {}
                            };
                        } catch(e) { stderr('[console patch error] ' + e); }
                    "#;
                    if let Err(e) = js_ctx.eval::<(), _>(console_js) {
                        eprintln!("[console patch error] {e:?}");
                    }
                    let sliders_rc = Rc::new(RefCell::new(Vec::new()));
                    let sliders_api = sliders_rc.clone();
                    let graph_lines_api = self.graph_lines.clone();
                    let add_slider = Func::from(move |name: String, params: rquickjs::Object| {
                        let min: f64 = params.get("min").unwrap_or(0.0);
                        let max: f64 = params.get("max").unwrap_or(1.0);
                        let step: f64 = params.get("step").unwrap_or(0.1);
                        let default: f64 = params.get("default").unwrap_or(0.0);
                        sliders_api.borrow_mut().push(SliderParam {
                            name: name.clone(),
                            min,
                            max,
                            step,
                            value: default
                        });
                    });
                    js_ctx.globals().set("addSlider", add_slider).unwrap();
                    let add_parametric_graph = Func::from(move |name: String, f: rquickjs::Function, range: rquickjs::Object| -> JsResult<()> {
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
                    let vectors_api = self.vectors.clone();
                    let add_vector = Func::from(move |name: String, start_f: rquickjs::Function, vec_f: rquickjs::Function, t: f64| -> JsResult<()> {
                        let start: Vec<f64> = start_f.call((t,))?;
                        let vec: Vec<f64> = vec_f.call((t,))?;
                        if start.len() == 2 && vec.len() == 2 {
                            vectors_api.borrow_mut().push((
                                name.clone(),
                                vec![[start[0], start[1]]],
                                vec![[start[0] + vec[0], start[1] + vec[1]]]
                            ));
                        }
                        Ok(())
                    });
                    js_ctx.globals().set("addVector", add_vector).unwrap();
                    // JSコードの評価
                    if let Err(e) = js_ctx.eval::<(), _>(self.js_code.as_str()) {
                        // 例外内容を取得して詳細エラー表示
                        let exc = js_ctx.catch();
                        let exc_str = format!("{:?}", exc);
                        js_error = Some(format!("JavaScript parse error: {e:?}\nException: {}", exc_str));
                        return;
                    }
                    // スライダーの初期化
                    let setup_exists = js_ctx.eval::<bool, _>("typeof setup === 'function'").unwrap_or(false);
                    if setup_exists {
                        if let Err(e) = js_ctx.eval::<(), _>("try {setup();} catch (e) {stderr(e.toString()+'\\n'+e.stack);}") {
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
                    self.vectors.borrow_mut().clear();  // ベクトルデータのクリア

                    // draw関数実行
                    if let Err(e) = js_ctx.eval::<(), _>("try {draw();} catch (e) {stderr(e.toString()+'\\n'+e.stack);}") {
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
