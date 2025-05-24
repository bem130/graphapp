use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints};
use egui::Color32;
use boa_engine::{Context as BoaContext, Source, JsResult, JsValue, JsArgs, NativeFunction, Context};
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

// チェックボックス情報を保持する構造体
#[derive(Clone)]
struct CheckboxParam {
    name: String,   // JSで参照する変数名
    label: String,  // チェックボックス横のラベル
    value: bool,    // 現在値
}

// カラーピッカー情報を保持する構造体
#[derive(Clone)]
struct ColorPickerParam {
    name: String,   // JSで参照する変数名
    value: Color32, // 現在の色 (egui::Color32)
}

// アプリケーションの状態を保持する構造体
struct ParametricPlotApp {
    sliders: Vec<SliderParam>,
    checkboxes: Vec<CheckboxParam>, // チェックボックス一覧を追加
    color_pickers: Vec<ColorPickerParam>, // カラーピッカー一覧を追加
    js_context: BoaContext,
    js_code_evaluated: bool,
    graph_lines: Rc<RefCell<Vec<(String, Vec<[f64; 2]>, Color32, f32)>>>, // (名前, 点群, 色, 太さ)
    vectors: Rc<RefCell<Vec<(String, Vec<[f64; 2]>, Vec<[f64; 2]>, Color32, f32)>>>,  // (名前, 始点群, 終点群, 色, 太さ)
    js_code: String, // JavaScriptエディタ用
    last_js_code: String, // 前回実行したJSコード
}

impl Default for ParametricPlotApp {
    fn default() -> Self {
        let mut js_context = BoaContext::default();
        let default_js_code = r#"
function setup() {
    addSlider('radius', { min: 0.5, max: 5.0, step: 0.1, default: 1.0 });
    addColorpicker('lineColor', { default: [255, 0, 0] });
    addCheckbox('show', '円を表示する', { default: true });
}
function draw() {
    if (show) {
        addParametricGraph(
            '円',
            function(t) { return [radius * Math.cos(t), radius * Math.sin(t)]; },
            { min: 0, max: 2 * Math.PI, num_points: 100 },
            { color: lineColor, weight: 2.0 }
        );
    }
}
"#.to_string();
        Self {
            sliders: Vec::new(),
            checkboxes: Vec::new(),
            color_pickers: Vec::new(),
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
        egui::SidePanel::right("js_editor_panel").min_width(600.0).show(ctx, |ui| {
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
                        .desired_width(2000.0) 
                        .code_editor()
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
                for (name, points, color, weight) in self.graph_lines.borrow().iter() {
                    let line = Line::new(name, PlotPoints::new(points.clone()))
                        .color(*color)
                        .width(*weight);
                    plot_ui.line(line);
                }

                // ベクトルをLine::newを使用して描画 (本体 + 矢じり2線)
                const ARROW_HEAD_LENGTH: f64 = 0.15; // 矢じりの各辺の長さ
                const ARROW_HEAD_ANGLE: f64 = std::f64::consts::PI / 7.0; // 矢じりの角度 (約25.7度)

                for (name, origins_vec, tips_vec, color, weight) in self.vectors.borrow().iter() {
                    // 現在のJSの実装では、origins_vecとtips_vecは常に要素を1つだけ持つ
                    if origins_vec.is_empty() || tips_vec.is_empty() {
                        continue;
                    }
                    let origin = origins_vec[0]; // [f64; 2]
                    let tip = tips_vec[0];       // [f64; 2]

                    // 1. ベクトルの本体を描画
                    let main_line_points = PlotPoints::new(vec![origin, tip]);
                    let main_line = Line::new(format!("{}_main", name), main_line_points)
                        .color(*color)
                        .width(*weight);
                    plot_ui.line(main_line);

                    // ベクトルの方向と長さを計算
                    let vec_dx = tip[0] - origin[0];
                    let vec_dy = tip[1] - origin[1];
                    let vec_len = (vec_dx.powi(2) + vec_dy.powi(2)).sqrt();

                    if vec_len < 1e-6 { // ベクトルが非常に短い場合は矢じりを描画しない
                        continue;
                    }

                    // 矢じりの長さを調整 (ベクトル本体が短い場合は矢じりも短くする)
                    let actual_arrow_head_length = ARROW_HEAD_LENGTH.min(vec_len * 0.4);

                    // 矢じりのための基準ベクトル（tipからoriginへ向かう方向）
                    let base_arrow_dx_norm = -vec_dx / vec_len;
                    let base_arrow_dy_norm = -vec_dy / vec_len;

                    // 矢じりの一方の辺の計算
                    let angle1 = ARROW_HEAD_ANGLE;
                    let cos_a1 = angle1.cos();
                    let sin_a1 = angle1.sin();
                    let arrow1_tip_dx = base_arrow_dx_norm * cos_a1 - base_arrow_dy_norm * sin_a1;
                    let arrow1_tip_dy = base_arrow_dx_norm * sin_a1 + base_arrow_dy_norm * cos_a1;
                    let arrow_p1 = [
                        tip[0] + actual_arrow_head_length * arrow1_tip_dx,
                        tip[1] + actual_arrow_head_length * arrow1_tip_dy,
                    ];

                    // 矢じりのもう一方の辺の計算
                    let angle2 = -ARROW_HEAD_ANGLE; // 反対側の角度
                    let cos_a2 = angle2.cos();
                    let sin_a2 = angle2.sin();
                    let arrow2_tip_dx = base_arrow_dx_norm * cos_a2 - base_arrow_dy_norm * sin_a2;
                    let arrow2_tip_dy = base_arrow_dx_norm * sin_a2 + base_arrow_dy_norm * cos_a2;
                    let arrow_p2 = [
                        tip[0] + actual_arrow_head_length * arrow2_tip_dx,
                        tip[1] + actual_arrow_head_length * arrow2_tip_dy,
                    ];

                    // 2. 矢じりの線1を描画
                    plot_ui.line(Line::new(format!("{}_arrow1", name), PlotPoints::new(vec![tip, arrow_p1])).color(*color).width(*weight));
                    // 3. 矢じりの線2を描画
                    plot_ui.line(Line::new(format!("{}_arrow2", name), PlotPoints::new(vec![tip, arrow_p2])).color(*color).width(*weight));
                }
            });

            // --- スライダー・チェックボックスを重ねて表示 ---
            if !self.sliders.is_empty() || !self.checkboxes.is_empty() || !self.color_pickers.is_empty() {
                // 左上にパラメータパネルを配置
                egui::Window::new("パラメータ")
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
                        for checkbox in &mut self.checkboxes {
                            if ui.checkbox(&mut checkbox.value, &checkbox.label).changed() {
                                need_redraw = true;
                            }
                        }
                        for picker in &mut self.color_pickers {
                            ui.horizontal(|ui| {
                                ui.label(&picker.name);
                                // Color32 (u8 0-255 per channel) を [f32; 3] (0.0-1.0 per channel) に変換
                                let mut color_f32 = [
                                    picker.value.r() as f32 / 255.0,
                                    picker.value.g() as f32 / 255.0,
                                    picker.value.b() as f32 / 255.0,
                                ];
                                if ui.color_edit_button_rgb(&mut color_f32).changed() {
                                    // [f32; 3] から Color32 に戻す (アルファは常に255)
                                    picker.value = Color32::from_rgb((color_f32[0] * 255.0) as u8, (color_f32[1] * 255.0) as u8, (color_f32[2] * 255.0) as u8);
                                    need_redraw = true;
                                }
                            });
                        }
                    });
            }

            // --- JavaScript関連の処理 ---
            // JSコードが変更された場合は再評価
            if js_code_changed || !self.js_code_evaluated || self.js_code != self.last_js_code {
                self.js_code_evaluated = false;
                self.last_js_code = self.js_code.clone();
                // Rust側stdout/stderrをJSに提供
                let stdout = |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let content = args.get_or_undefined(0).to_string(context)?;
                    println!("{}", format!("{:?}",content));
                    Ok(JsValue::undefined())
                };
                self.js_context.register_global_builtin_callable("stdout".into(), 1, NativeFunction::from_copy_closure(stdout)).unwrap();

                // stderr
                let stderr = |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let content = args.get_or_undefined(0).to_string(context)?;
                    eprintln!("{}", format!("{:?}",content).red());
                    Ok(JsValue::undefined())
                };
                self.js_context.register_global_builtin_callable("stderr".into(), 1, NativeFunction::from_copy_closure(stderr)).unwrap();

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
                    self.js_context.eval(Source::from_bytes(console_js));

                    let sliders_rc = Rc::new(RefCell::new(Vec::new()));
                    let sliders_api = sliders_rc.clone();
                    let checkboxes_rc = Rc::new(RefCell::new(Vec::new()));
                    let checkboxes_api = checkboxes_rc.clone();
                    let color_pickers_rc = Rc::new(RefCell::new(Vec::new()));
                    let color_pickers_api = color_pickers_rc.clone();
                    let graph_lines_api = self.graph_lines.clone();
                    let add_slider = |_this: &JsValue, args: &[JsValue], context: &mut Context| {// 名前（args[0]）を文字列として抽出
                        let name = args.get_or_undefined(0).to_string(context)?;
                        // パラメータ（args[1]）をオブジェクトとして抽出
                        let params = args.get_or_undefined(1).to_object(context)?;
                        // パラメータからプロパティを抽出
                        let min = params.get("min".into(), context).and_then(|v| v.to_number(context)).map(|num| if num.is_nan() { 0.0 } else { num }).unwrap_or(0.0);
                        let max = params.get("max".into(), context).and_then(|v| v.to_number(context)).map(|num| if num.is_nan() { 1.0 } else { num }).unwrap_or(1.0);
                        let step = params.get("step".into(), context).and_then(|v| v.to_number(context)).map(|num| if num.is_nan() { 0.1 } else { num }).unwrap_or(0.1);
                        let default = params.get("default".into(), context).and_then(|v| v.to_number(context)).map(|num| if num.is_nan() { 0.0 } else { num }).unwrap_or(0.0);
                        // 共有状態に追加（例: sliders_rc）
                        sliders_rc.borrow_mut().push(SliderParam {
                            name: format!("{:?}",name),
                            min,
                            max,
                            step,
                            value: default,
                        });
                        Ok(JsValue::undefined())
                    };
                    self.js_context.register_global_builtin_callable("addSlider".into(), 2, NativeFunction::from_copy_closure(add_slider)).unwrap();
                    // addCheckbox API
                    let add_checkbox = Func::from(move |name: String, label: String, params: Option<rquickjs::Object>| {
                        let default = params.as_ref().and_then(|p| p.get("default").ok()).unwrap_or(true);
                        checkboxes_api.borrow_mut().push(CheckboxParam {
                            name: name.clone(),
                            label: label.clone(),
                            value: default,
                        });
                    });
                    js_ctx.globals().set("addCheckbox", add_checkbox).unwrap();
                    // addColorpicker API
                    let add_color_picker = Func::from(move |name: String, params: Option<rquickjs::Object>| {
                        let mut default_color_val = Color32::from_rgb(255, 255, 255); // デフォルトは不透明の白
                        if let Some(p_obj) = params {
                            if let Ok(color_array) = p_obj.get::<_, rquickjs::Array>("default") {
                                if color_array.len() >= 3 {
                                    let r = color_array.get::<u8>(0).unwrap_or(255);
                                    let g = color_array.get::<u8>(1).unwrap_or(255);
                                    let b = color_array.get::<u8>(2).unwrap_or(255);
                                    // アルファ値は無視し、常に不透明 (from_rgbがA=255とする)
                                    default_color_val = Color32::from_rgb(r, g, b);
                                }
                            }
                        }
                        color_pickers_api.borrow_mut().push(ColorPickerParam {
                            name: name.clone(),
                            value: default_color_val,
                        });
                    });
                    js_ctx.globals().set("addColorpicker", add_color_picker).unwrap();
                    let add_parametric_graph = Func::from(move |name: String, f: rquickjs::Function, range: rquickjs::Object, style: Option<rquickjs::Object>| -> JsResult<()> {
                        let min: f64 = range.get("min").unwrap_or(0.0);
                        let max: f64 = range.get("max").unwrap_or(2.0 * std::f64::consts::PI);
                        let delta: Option<f64> = range.get("delta").ok();
                        let mut points = Vec::new();

                        const DEFAULT_GRAPH_COLOR: Color32 = Color32::from_rgb(200, 100, 0); // 不透明
                        const DEFAULT_GRAPH_WEIGHT: f32 = 1.5;
                        let mut line_color = DEFAULT_GRAPH_COLOR;
                        let mut line_weight = DEFAULT_GRAPH_WEIGHT;

                        if let Some(style_obj) = style {
                            if let Ok(color_array) = style_obj.get::<_, rquickjs::Array>("color") {
                                if color_array.len() >= 3 {
                                    let r = color_array.get::<u8>(0).unwrap_or(DEFAULT_GRAPH_COLOR.r());
                                    let g = color_array.get::<u8>(1).unwrap_or(DEFAULT_GRAPH_COLOR.g());
                                    let b = color_array.get::<u8>(2).unwrap_or(DEFAULT_GRAPH_COLOR.b());
                                    // アルファ値は無視し、常に不透明
                                    line_color = Color32::from_rgb(r, g, b);
                                }
                            }
                            line_weight = style_obj.get("weight").unwrap_or(DEFAULT_GRAPH_WEIGHT);
                        }

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
                        graph_lines_api.borrow_mut().push((name, points, line_color, line_weight));
                        Ok(())
                    });
                    js_ctx.globals().set("addParametricGraph", add_parametric_graph).unwrap();
                    let vectors_api = self.vectors.clone();
                    let add_vector = Func::from(move |name: String, start_f: rquickjs::Function, vec_f: rquickjs::Function, t: f64, style: Option<rquickjs::Object>| -> JsResult<()> {
                        let start: Vec<f64> = start_f.call((t,))?;
                        let vec: Vec<f64> = vec_f.call((t,))?;

                        const DEFAULT_ARROW_COLOR: Color32 = Color32::from_rgb(0, 150, 200); // 不透明
                        const DEFAULT_ARROW_WEIGHT: f32 = 1.5;
                        let mut arrow_color = DEFAULT_ARROW_COLOR;
                        let mut arrow_weight = DEFAULT_ARROW_WEIGHT;

                        if let Some(style_obj) = style {
                            if let Ok(color_array) = style_obj.get::<_, rquickjs::Array>("color") {
                                if color_array.len() >= 3 {
                                    let r = color_array.get::<u8>(0).unwrap_or(DEFAULT_ARROW_COLOR.r());
                                    let g = color_array.get::<u8>(1).unwrap_or(DEFAULT_ARROW_COLOR.g());
                                    let b = color_array.get::<u8>(2).unwrap_or(DEFAULT_ARROW_COLOR.b());
                                    // アルファ値は無視し、常に不透明
                                    arrow_color = Color32::from_rgb(r, g, b);
                                }
                            }
                            arrow_weight = style_obj.get("weight").unwrap_or(DEFAULT_ARROW_WEIGHT);
                        }

                        if start.len() == 2 && vec.len() == 2 {
                            vectors_api.borrow_mut().push((
                                name.clone(),
                                vec![[start[0], start[1]]],
                                vec![[start[0] + vec[0], start[1] + vec[1]]],
                                arrow_color,
                                arrow_weight
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
                        self.checkboxes = checkboxes_rc.borrow().clone();  // チェックボックスの初期化
                        self.color_pickers = color_pickers_rc.borrow().clone(); // カラーピッカーの初期化
                        need_redraw = true;
                    } else {
                        js_error = Some("setup関数が定義されていません".to_string());
                        return;
                    }
                    self.js_code_evaluated = true;
                }

                // 初期表示時またはスライダー・チェックボックス変更時に再描画
                if need_redraw || self.graph_lines.borrow().is_empty() {
                    // スライダー値をJSグローバルに注入
                    for slider in &self.sliders {
                        js_ctx.globals().set(slider.name.as_str(), slider.value).unwrap();
                    }
                    // チェックボックス値をJSグローバルに注入
                    for checkbox in &self.checkboxes {
                        js_ctx.globals().set(checkbox.name.as_str(), checkbox.value).unwrap();
                    }
                    // カラーピッカー値をJSグローバルに注入 ([r,g,b] の3要素配列として)
                    for picker in &self.color_pickers {
                        let js_array = match rquickjs::Array::new(js_ctx.clone()) {
                            Ok(arr) => arr,
                            Err(e) => {
                                js_error = Some(format!("Failed to create JS array for color picker {}: {:?}", picker.name, e));
                                return;
                            }
                        };
                        if let Err(e) = js_array.set(0, picker.value.r()) {
                            js_error = Some(format!("Failed to set R for color picker {}: {:?}", picker.name, e));
                            return;
                        }
                        if let Err(e) = js_array.set(1, picker.value.g()) {
                            js_error = Some(format!("Failed to set G for color picker {}: {:?}", picker.name, e));
                            return;
                        }
                        if let Err(e) = js_array.set(2, picker.value.b()) {
                            js_error = Some(format!("Failed to set B for color picker {}: {:?}", picker.name, e));
                            return;
                        }
                        // アルファ値は注入しない
                        if let Err(e) = js_ctx.globals().set(picker.name.as_str(), js_array) {
                            js_error = Some(format!("Failed to set JS global for color picker {}: {:?}", picker.name, e));
                            return;
                        }
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
