use boa_engine::JsObject;
use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints, Polygon, Stroke};
use egui::Color32;
use boa_engine::{Context as BoaContext, Source, JsValue, JsArgs, NativeFunction, js_string, property::Attribute, property::PropertyKey, JsNativeError};
use egui::{Ui, Widget, Response, Sense, Pos2, Rect, Stroke, TextEdit, Slider};
use egui_commonmark;
use egui_extras::syntax_highlighting;
#[cfg(target_arch = "wasm32")]
use hframe::Aware;
use base64::{engine::general_purpose, Engine as _};
use form_urlencoded::{parse, Serializer};
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
static PENDING_CONTENT: Mutex<Option<String>> = Mutex::new(None);


#[cfg(not(target_arch = "wasm32"))]
use colored::Colorize;

// WASMでのログ出力設定
#[cfg(target_arch = "wasm32")]
pub fn setup_logging() {
    console_log::init_with_level(log::Level::Debug).expect("error initializing log");
}

// ネイティブでのログ出力設定
#[cfg(not(target_arch = "wasm32"))]
pub fn setup_logging() {
    env_logger::init();
}

use std::cell::RefCell;
use std::rc::Rc;

// スライダ情報を保持する構造体
#[derive(Clone)]
pub struct SliderParam {
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

// ログメッセージの種類と内容
#[derive(Clone, Debug, PartialEq)] // PartialEqを追加して比較できるようにする
enum LogType {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, PartialEq)] // PartialEqを追加
struct LogEntry {
    log_type: LogType,
    message: String,
}

// アプリケーションの状態を保持する構造体
pub struct ParametricPlotApp {
    sliders: Vec<SliderParam>,
    checkboxes: Vec<CheckboxParam>, // チェックボックス一覧を追加
    color_pickers: Vec<ColorPickerParam>, // カラーピッカー一覧を追加
    js_context: BoaContext,
    js_code_evaluated: bool,
    graph_lines: Rc<RefCell<Vec<(String, Vec<[f64; 2]>, Color32, f32)>>>, // (名前, 点群, 色, 太さ)
    vectors: Rc<RefCell<Vec<(String, Vec<[f64; 2]>, Vec<[f64; 2]>, Color32, f32)>>>,  // (名前, 始点群, 終点群, 色, 太さ)
    js_code: String, // JavaScriptエディタ用
    last_js_code: String, // 前回実行したJSコード
    api_docs_content: String,
    log_output: Rc<RefCell<Vec<LogEntry>>>,
    commonmark_cache: egui_commonmark::CommonMarkCache,
    polygons: Rc<RefCell<Vec<(String, Vec<[f64; 2]>, Color32, Color32, f32)>>>, // (名前, 頂点群, 枠線色, 塗りつぶし色, 線の太さ)
}

impl Default for ParametricPlotApp {
    fn default() -> Self {
        let js_context = BoaContext::default();
        let mut default_js_code = r#"
function setup() {
    addSlider('radius', { min: 0.5, max: 5.0, step: 0.001, default: 1.0 });
    addColorpicker('lineColor', { default: [255, 0, 0] });
    addColorpicker('fillColor', { default: [255, 200, 200] });
    addCheckbox('show', '図形を表示する', { default: true });
}

function draw() {
    if (show) {
        // 円の描画
        addParametricGraph(
            '円',
            function(t) { return [radius * Math.cos(t), radius * Math.sin(t)]; },
            { min: 0, max: 2 * Math.PI, num_points: 100 },
            { color: lineColor, weight: 2.0 }
        );

        // 三角形の描画
        addPolygon(
            '三角形',
            [[radius, 0], [radius * Math.cos(2*Math.PI/3), radius * Math.sin(2*Math.PI/3)], [radius * Math.cos(4*Math.PI/3), radius * Math.sin(4*Math.PI/3)]],
            { color: lineColor, fill: fillColor, weight: 2.0 }
        );
    }
}
"#.to_string();
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::Url;
            if let Some(window) = web_sys::window() {
                if let Ok(href) = window.location().href() {
                    if let Ok(mut url) = Url::new(&href) {
                        // クエリをパース
                        let search = url.search();
                        let mut code_val = None;
                        if !search.is_empty() {
                            for (k, v) in parse(search.trim_start_matches('?').as_bytes()) {
                                if k == "code" {
                                    // base64デコード→utf8
                                    if let Ok(decoded_b64) = general_purpose::URL_SAFE_NO_PAD.decode(&*v) {
                                        if let Ok(decoded_str) = String::from_utf8(decoded_b64) {
                                            code_val = Some(decoded_str);
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(code) = code_val {
                            if !code.trim().is_empty() {
                                default_js_code = code;
                            }
                        }
                    }
                }
            }
            update_monaco(&default_js_code);
        }
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
            api_docs_content: include_str!("../doc/api.md").to_string(),
            log_output: Rc::new(RefCell::new(Vec::new())),
            commonmark_cache: egui_commonmark::CommonMarkCache::default(),
            polygons: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl ParametricPlotApp {}

impl App for ParametricPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut js_code_changed = false;


        #[cfg(target_arch = "wasm32")]
        if let Ok(mut content) = PENDING_CONTENT.try_lock() {
            match content.clone() {
                Some(x) => {
                    self.js_code = x;
                    js_code_changed = true;
                    *content = None;
                },
                _ => {}
            }
        }

        // API Documentationウィンドウを常に表示
        let subwin = egui::Window::new("API Documentation")
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Copy API Docs").clicked() {
                        ui.ctx().copy_text(self.api_docs_content.clone());
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui_commonmark::CommonMarkViewer::new()
                        .show(ui, &mut self.commonmark_cache, &self.api_docs_content);
                });
            });
        #[cfg(target_arch = "wasm32")]
        subwin.aware();

        #[cfg(not(target_arch = "wasm32"))]
        let subwin = egui::Window::new("Javascript Editor").min_width(600.0).show(ctx, |ui| {
            let mut theme = syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
            if ui.button("再実行").clicked() {
                js_code_changed = true;
            }
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
            // ScrollAreaがSidePanelの残りのスペースを埋めるようにする
            // auto_shrink([false, false]) で、利用可能なスペースいっぱいに広がる
            egui::ScrollArea::vertical()
                .auto_shrink([false, false]) // 水平・垂直ともに利用可能なスペースを埋める
                .show(ui, |ui| { // このuiはScrollAreaのコンテンツ領域のui
                    // TextEditをScrollAreaのコンテンツ領域いっぱいに広げる
                    let text_edit_widget = egui::TextEdit::multiline(&mut self.js_code)
                            .font(egui::TextStyle::Monospace)
                            // .desired_width(f32::INFINITY) // add_sizedで幅も指定するため、ここでは必須ではない
                            .code_editor()
                            .layouter(&mut layouter);
                    // TextEditウィジェットを、ScrollAreaの利用可能なサイズいっぱいに配置
                    let response = ui.add_sized(ui.available_size(), text_edit_widget);
                if response.changed() {
                    js_code_changed = true;
                    #[cfg(target_arch = "wasm32")]
                    update_monaco(&self.js_code);
                }
            });
        });
        // #[cfg(target_arch = "wasm32")]
        // subwin.aware();
        egui::CentralPanel::default().show(ctx, |ui| {            let mut need_redraw = false;

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
                // 多角形を描画
                for (name, points, color, fill, weight) in self.polygons.borrow().iter() {
                    let polygon = Polygon::new(name, PlotPoints::new(points.clone()))
                        .stroke(Stroke::new(*weight, *color))
                        .fill_color(*fill);
                    plot_ui.polygon(polygon);
                }

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
                let subwin = egui::Window::new("パラメータ")
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.set_min_width(100.0);
                        for slider in &mut self.sliders {
                            let widget = CustomSlider::new(slider);
                            if ui.add(widget).changed() {
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
                #[cfg(target_arch = "wasm32")]
                subwin.aware();
            }

            // --- ログ出力ウィンドウ ---
            let subwin = egui::Window::new("出力ログ")
                .default_size([600.0, 200.0]) // デフォルトサイズを調整
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::both().auto_shrink([false,false]).max_height(300.0).show(ui, |ui| {
                        let logs = self.log_output.borrow();
                        if logs.is_empty() {
                            ui.label("ログ出力はありません。");
                            return;
                        }

                        let mut display_entries: Vec<(LogEntry, usize)> = Vec::new();
                        if !logs.is_empty() {
                            display_entries.push((logs[0].clone(), 1));
                            for i in 1..logs.len() {
                                let current_log = &logs[i];
                                let last_display_entry = display_entries.last_mut().unwrap();
                                // メッセージタイプと内容が同じ場合にカウントアップ
                                if current_log.log_type == last_display_entry.0.log_type && current_log.message == last_display_entry.0.message {
                                    last_display_entry.1 += 1;
                                } else {
                                    display_entries.push((current_log.clone(), 1));
                                }
                            }
                        }

                        for (entry, count) in display_entries.iter() {
                            let mut text = egui::RichText::new(format!("[{}] x{}\n{}",
                                match entry.log_type {
                                    LogType::Stdout => "stdout",
                                    LogType::Stderr => "stderr",
                                },
                                count,
                                entry.message,
                            ));
                            if entry.log_type == LogType::Stderr {
                                text = text.color(Color32::RED);
                            }
                            ui.label(text); // 常にRichTextを使用するように変更 (count > 1 の条件を削除)
                        }});
                });
            #[cfg(target_arch = "wasm32")]
            subwin.aware();

            // --- JavaScript関連の処理 ---
            // JSコードが変更された場合は再評価
            if js_code_changed || !self.js_code_evaluated || self.js_code != self.last_js_code {
                #[cfg(target_arch = "wasm32")]
                {
                    use web_sys::Url;
                    if let Some(window) = web_sys::window() {
                        if let Ok(href) = window.location().href() {
                            if let Ok(url) = Url::new(&href) {
                                // 既存クエリをパース
                                let search = url.search();
                                let mut params: Vec<(String, String)> = vec![];
                                if !search.is_empty() {
                                    for (k, v) in form_urlencoded::parse(search.trim_start_matches('?').as_bytes()) {
                                        if k != "code" {
                                            params.push((k.into_owned(), v.into_owned()));
                                        }
                                    }
                                }
                                // codeパラメータをbase64で追加
                                let code_b64 = general_purpose::URL_SAFE_NO_PAD.encode(&self.js_code);
                                params.push(("code".to_string(), code_b64));
                                // クエリ再構築
                                let mut ser = Serializer::new(String::new());
                                for (k, v) in params {
                                    ser.append_pair(&k, &v);
                                }
                                let new_query = ser.finish();
                                url.set_search(&format!("?{}", new_query));
                                let _ = window.history().expect("history").replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url.href()));
                            }
                        }
                    }
                }

                self.js_code_evaluated = false;
                self.last_js_code = self.js_code.clone();

                // Rust側stdout/stderrをJSに提供
                let log_output_stdout = self.log_output.clone();
                let stdout = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let content = args.get_or_undefined(0).to_string(context)?;
                    let msg = content.to_std_string().unwrap_or_else(|e| format!("[stdout conversion error: {:?}]", e));
                    println!("[JS stdout]: {}", msg); // Keep original console log
                    log_output_stdout.borrow_mut().push(LogEntry {
                        log_type: LogType::Stdout,
                        message: msg,
                    });
                    Ok(JsValue::undefined())
                };
                unsafe {
                    self.js_context.register_global_builtin_callable("stdout".into(), 1, NativeFunction::from_closure(stdout)).unwrap();
                }

                // stderr
                let log_output_stderr = self.log_output.clone();
                let stderr = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let content = args.get_or_undefined(0).to_string(context)?;
                    let msg = content.to_std_string().unwrap_or_else(|e| format!("[stderr conversion error: {:?}]", e));
                    #[cfg(not(target_arch = "wasm32"))]
                    eprintln!("[JS stderr]: {}", msg.clone().red());
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::error_1(&msg.clone().into());
                    log_output_stderr.borrow_mut().push(LogEntry {
                        log_type: LogType::Stderr,
                        message: msg,
                    });
                    Ok(JsValue::undefined())
                };
                unsafe {
                    self.js_context.register_global_builtin_callable("stderr".into(), 1, NativeFunction::from_closure(stderr)).unwrap();
                }

                // JS側でconsole.log/console.errorをstdout/stderr経由でJSON出力するように定義
                let console_js = r#"
                    try {
                        if (typeof globalThis.console !== 'object' || globalThis.console === null) {
                            globalThis.console = {};
                        }
                        globalThis.console.log = function(...args) {
                            try { stdout(args.map(x=>JSON.stringify(x)).join(" ")); } catch(e) {}
                        };
                        globalThis.console.error = function(...args) {
                            try { stderr(args.map(x=>JSON.stringify(x)).join(" ")); } catch(e) {}
                        };
                    } catch(e) { stderr('[console patch error] ' + e); }
                "#;
                if let Err(e) = self.js_context.eval(Source::from_bytes(console_js)) {
                    println!("Error setting up console: {:?}", e);
                }

                let sliders_rc = Rc::new(RefCell::new(Vec::new()));
                let sliders_api = sliders_rc.clone();
                let checkboxes_rc = Rc::new(RefCell::new(Vec::new()));
                let checkboxes_api = checkboxes_rc.clone();
                let color_pickers_rc = Rc::new(RefCell::new(Vec::new()));
                let color_pickers_api = color_pickers_rc.clone();
                let graph_lines_api = self.graph_lines.clone();
                let vectors_api = self.vectors.clone();
                let polygons_api = self.polygons.clone();

                // addSlider API
                let add_slider = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let name = args.get_or_undefined(0).to_string(context)?;
                    let params = args.get_or_undefined(1).to_object(context)?;
                    
                    let min = params.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("min")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0);
                    let max = params.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("max")), context).and_then(|v| v.to_number(context)).unwrap_or(1.0);
                    let step = params.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("step")), context).and_then(|v| v.to_number(context)).unwrap_or(0.1);
                    let default = params.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("default")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0);
                    
                    sliders_api.borrow_mut().push(SliderParam {
                        name: name.to_std_string().unwrap(),
                        min,
                        max,
                        step,
                        value: default,
                    });
                    Ok(JsValue::undefined())
                };
                unsafe { self.js_context.register_global_builtin_callable("addSlider".into(), 2, NativeFunction::from_closure(add_slider)).unwrap(); }

                // addCheckbox API
                let add_checkbox = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let name = args.get_or_undefined(0).to_string(context)?;
                    let label = args.get_or_undefined(1).to_string(context)?;
                    let params = args.get_or_undefined(2).to_object(context)?;
                    
                    let default = if let Ok(default_val) = params.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("default")), context) {
                        default_val.as_boolean().unwrap_or(true)
                    } else {
                        true
                    };

                    checkboxes_api.borrow_mut().push(CheckboxParam {
                        name: name.to_std_string().unwrap(),
                        label: label.to_std_string().unwrap(),
                        value: default,
                    });
                    Ok(JsValue::undefined())
                };
                unsafe { self.js_context.register_global_builtin_callable("addCheckbox".into(), 3, NativeFunction::from_closure(add_checkbox)).unwrap(); }

                // addColorpicker API
                let add_color_picker = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let name = args.get_or_undefined(0).to_string(context)?;
                    let params = args.get_or_undefined(1).to_object(context)?;
                    let mut default_color_val = Color32::from_rgb(255, 255, 255); // デフォルトは白
                    if let Ok(js_val) = params.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("default")), context) {
                        if let Some(obj) = js_val.as_object() {
                            if obj.is_array() {
                                let len = obj.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("length")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0) as usize;
                                if len >= 3 {
                                    let r = obj.get(0, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(255.0) as u8;
                                    let g = obj.get(1, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(255.0) as u8;
                                    let b = obj.get(2, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(255.0) as u8;
                                    default_color_val = Color32::from_rgb(r, g, b);
                                }
                            }
                        }
                    }
                    color_pickers_api.borrow_mut().push(ColorPickerParam {
                        name: name.to_std_string().unwrap(),
                        value: default_color_val,
                    });
                    Ok(JsValue::undefined())
                };
                unsafe { self.js_context.register_global_builtin_callable("addColorpicker".into(), 2, NativeFunction::from_closure(add_color_picker)).unwrap(); }

                // addParametricGraph API
                let add_parametric_graph = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let name = args.get_or_undefined(0).to_string(context)?;
                    let f = args.get_or_undefined(1).as_object()
                        .ok_or_else(|| boa_engine::JsNativeError::typ().with_message("Second argument must be a function"))?;
                    let range = args.get_or_undefined(2).to_object(context)?;
                    let style = args.get_or_undefined(3).to_object(context)?;                    let min: f64 = range.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("min")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0);
                    let max: f64 = range.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("max")), context).and_then(|v| v.to_number(context)).unwrap_or(2.0 * std::f64::consts::PI);
                    let num_points: f64 = range.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("num_points")), context).and_then(|v| v.to_number(context)).unwrap_or(500.0);
                    // deltaは必ず計算する（rangeから取得しない）
                    let delta: f64 = (max - min) / num_points;
                    const DEFAULT_GRAPH_COLOR: Color32 = Color32::from_rgb(200, 100, 0);
                    const DEFAULT_GRAPH_WEIGHT: f32 = 1.5;
                    let mut line_color = DEFAULT_GRAPH_COLOR;
                    let mut line_weight = DEFAULT_GRAPH_WEIGHT;
                    if let Ok(js_val) = style.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("color")), context) {
                        if let Some(obj) = js_val.as_object() {
                            if obj.is_array() {
                                let len = obj.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("length")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0) as usize;
                                if len >= 3 {
                                    let r = obj.get(0, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(200.0) as u8;
                                    let g = obj.get(1, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(100.0) as u8;
                                    let b = obj.get(2, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0) as u8;
                                    line_color = Color32::from_rgb(r, g, b);
                                }
                            }
                        }
                    }
                    if let Ok(weight) = style.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("weight")), context) {
                        line_weight = weight.to_number(context).unwrap_or(DEFAULT_GRAPH_WEIGHT as f64) as f32;
                    }                    let mut points = Vec::with_capacity(num_points as usize);
                    let mut t = min;
                    // 最後の点まで確実に生成するためのループ
                    for _ in 0..=num_points as usize {
                        let args = [JsValue::from(t)];
                        if let Ok(result) = f.call(&JsValue::undefined(), &args, context) {
                            if let Some(obj) = result.as_object() {
                                if obj.is_array() {
                                    let len = obj.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("length")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0) as usize;
                                    if len >= 2 {
                                        let x = obj.get(0, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0);
                                        let y = obj.get(1, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0);
                                        points.push([x, y]);
                                    }
                                }
                            }
                        }
                        t += delta;
                    }
                    graph_lines_api.borrow_mut().push((name.to_std_string().unwrap(), points, line_color, line_weight));
                    Ok(JsValue::undefined())
                };
                unsafe { self.js_context.register_global_builtin_callable("addParametricGraph".into(), 4, NativeFunction::from_closure(add_parametric_graph)).unwrap(); }

                // addVector API (api.md仕様)
                let add_vector = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| {
                    let name = args.get_or_undefined(0).to_string(context)?;
                    let start_func = args.get_or_undefined(1).as_object()
                        .ok_or_else(|| boa_engine::JsNativeError::typ().with_message("Second argument must be a function"))?;
                    let vec_func = args.get_or_undefined(2).as_object()
                        .ok_or_else(|| boa_engine::JsNativeError::typ().with_message("Third argument must be a function"))?;
                    let t = args.get_or_undefined(3).to_number(context)?;
                    let style = args.get_or_undefined(4).to_object(context)?;
                    // デフォルト色・太さ
                    let mut color = Color32::from_rgb(0, 150, 200);
                    let mut weight = 1.5;
                    if let Ok(js_val) = style.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("color")), context) {
                        if let Some(obj) = js_val.as_object() {
                            if obj.is_array() {
                                let len = obj.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("length")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0) as usize;
                                if len >= 3 {
                                    let r = obj.get(0, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0) as u8;
                                    let g = obj.get(1, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(150.0) as u8;
                                    let b = obj.get(2, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(200.0) as u8;
                                    color = Color32::from_rgb(r, g, b);
                                }
                            }
                        }
                    }
                    if let Ok(weight_val) = style.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("weight")), context) {
                        weight = weight_val.to_number(context).unwrap_or(1.5) as f32;
                    }
                    // tで関数を呼び出し
                    let args_t = [JsValue::from(t)];
                    let start = if let Ok(result) = start_func.call(&JsValue::undefined(), &args_t, context) {
                        if let Some(obj) = result.as_object() {
                            if obj.is_array() {
                                let len = obj.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("length")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0) as usize;
                                if len >= 2 {
                                    let x = obj.get(0, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0);
                                    let y = obj.get(1, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0);
                                    Some([x, y])
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None };
                    let vec = if let Ok(result) = vec_func.call(&JsValue::undefined(), &args_t, context) {
                        if let Some(obj) = result.as_object() {
                            if obj.is_array() {
                                let len = obj.get(boa_engine::property::PropertyKey::from(boa_engine::JsString::from("length")), context).and_then(|v| v.to_number(context)).unwrap_or(0.0) as usize;
                                if len >= 2 {
                                    let dx = obj.get(0, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0);
                                    let dy = obj.get(1, context).unwrap_or(JsValue::undefined()).to_number(context).unwrap_or(0.0);
                                    Some([dx, dy])
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None };
                    let mut origins_vec = Vec::new();
                    let mut tips_vec = Vec::new();
                    if let (Some(start), Some(vec)) = (start, vec) {
                        origins_vec.push(start);
                        tips_vec.push([start[0] + vec[0], start[1] + vec[1]]);
                    }
                    vectors_api.borrow_mut().push((name.to_std_string().unwrap(), origins_vec, tips_vec, color, weight));
                    Ok(JsValue::undefined())
                };
                unsafe { self.js_context.register_global_builtin_callable("addVector".into(), 5, NativeFunction::from_closure(add_vector)).unwrap(); }

                // addPolygon API
                let add_polygon = move |_this: &JsValue, args: &[JsValue], context: &mut BoaContext| -> Result<JsValue, _> {
                    let name = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap_or_default();                    let points_array = args.get_or_undefined(1).as_object()
                        .ok_or_else(|| JsNativeError::error().with_message("Expected array for points"))?;
                    let style_obj = args.get_or_undefined(2).as_object();

                    let mut points = Vec::new();                    let length = {
                        let key: PropertyKey = js_string!("length").into();
                        points_array.get(key, context)?.as_number().unwrap_or(0.0) as u32
                    };
                    for i in 0..length {
                        if let Some(point) = points_array.get(i, context)?.as_object() {
                            let x = point.get(0, context)?.as_number().unwrap_or(0.0);
                            let y = point.get(1, context)?.as_number().unwrap_or(0.0);
                            points.push([x, y]);
                        }
                    }

                    let default_color = Color32::from_rgb(0, 0, 0);
                    let default_fill = Color32::from_rgb(128, 128, 255);
                    let default_weight = 1.5;                    let (color, fill, weight) = if let Some(style) = style_obj {                        let color = {
                            let key: PropertyKey = js_string!("color").into();
                            if let Some(color_arr) = style.get(key, context)?.as_object() {
                                Color32::from_rgb(
                                    color_arr.get(0, context)?.as_number().unwrap_or(0.0) as u8,
                                    color_arr.get(1, context)?.as_number().unwrap_or(0.0) as u8,
                                    color_arr.get(2, context)?.as_number().unwrap_or(0.0) as u8,
                                )
                            } else {
                                default_color
                            }
                        };

                        let fill = {
                            let key: PropertyKey = js_string!("fill").into();
                            if let Some(fill_arr) = style.get(key, context)?.as_object() {
                                Color32::from_rgb(
                                    fill_arr.get(0, context)?.as_number().unwrap_or(0.0) as u8,
                                    fill_arr.get(1, context)?.as_number().unwrap_or(0.0) as u8,
                                    fill_arr.get(2, context)?.as_number().unwrap_or(0.0) as u8,
                                )
                            } else {
                                default_fill
                            }
                        };

                        let weight = {
                            let key: PropertyKey = js_string!("weight").into();
                            style.get(key, context)?.as_number().unwrap_or(default_weight as f64) as f32
                        };

                        (color, fill, weight)
                    } else {
                        (default_color, default_fill, default_weight)
                    };

                    polygons_api.borrow_mut().push((name, points, color, fill, weight));
                    Ok(JsValue::undefined())
                };

                unsafe {
                    self.js_context.register_global_builtin_callable("addPolygon".into(), 3, NativeFunction::from_closure(add_polygon)).unwrap();
                }

                // ログ出力をリセット
                self.log_output.borrow_mut().clear();

                // コードの読み込み
                println!("load");
                if let Err(e) = self.js_context.eval(Source::from_bytes(self.js_code.as_str())) {
                    println!("Load error: {:?}", e);
                }

                // Setup関数の実行
                println!("setup");
                if let Err(e) = self.js_context.eval(Source::from_bytes("try {setup();} catch (e) {stderr(e.toString()+'\\n'+e.stack);}")) {
                    println!("Setup error: {:?}", e);
                }

                self.sliders = sliders_rc.borrow().clone();
                self.checkboxes = checkboxes_rc.borrow().clone();
                self.color_pickers = color_pickers_rc.borrow().clone();
                need_redraw = true;

                self.js_code_evaluated = true;
            }

            // グラフの再描画フラグ
            if need_redraw || self.graph_lines.borrow().is_empty() {
                // UI値をグローバル変数として注入
                for slider in &self.sliders {
                    self.js_context.register_global_property::<PropertyKey, f64>(js_string!(slider.name.clone()).into(), slider.value, Attribute::all()).ok();
                }
                for checkbox in &self.checkboxes {
                    self.js_context.register_global_property::<PropertyKey, bool>(js_string!(checkbox.name.clone()).into(), checkbox.value, Attribute::all()).ok();
                }
                for picker in &self.color_pickers {
                    // Array(3)をグローバルから作成
                    let global = self.js_context.global_object();
                    let array_ctor = global.get(js_string!("Array"), &mut self.js_context).unwrap();
                    let array_constructor = array_ctor.as_constructor().expect("Array is not a constructor");
                    let arr = array_constructor.construct(&[JsValue::from(3)], None, &mut self.js_context).unwrap();

                    // 配列に色の値をセット
                    arr.set(0, JsValue::from(picker.value.r() as u32), false, &mut self.js_context).ok();
                    arr.set(1, JsValue::from(picker.value.g() as u32), false, &mut self.js_context).ok();
                    arr.set(2, JsValue::from(picker.value.b() as u32), false, &mut self.js_context).ok();

                    // グローバル変数として設定
                    self.js_context.register_global_property::<PropertyKey, JsObject>(js_string!(picker.name.clone()).into(), arr, Attribute::all()).ok();
                }

                self.graph_lines.borrow_mut().clear();
                self.vectors.borrow_mut().clear();
                self.polygons.borrow_mut().clear();
                if let Err(e) = self.js_context.eval(Source::from_bytes("try {draw();} catch (e) {stderr(e.toString()+'\\n'+e.stack);}")) {
                    println!("Draw error: {:?}", e);
                }
            }
        });

        #[cfg(target_arch = "wasm32")]
        hframe::HtmlWindow::new("Monaco Editor").content("").show(ctx);
        #[cfg(target_arch = "wasm32")]
        hframe::sync(ctx);
        ctx.request_repaint();
    }
}



#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn update(data: &str) {
    if let Ok(mut content) = PENDING_CONTENT.try_lock() {
        *content = Some(data.to_string());
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/src/api.js")]
extern "C" {
    #[wasm_bindgen(js_name = update)]
    pub fn update_monaco(data: &str);
}

struct CustomSlider<'a> {
    param: &'a mut SliderParam,
}

impl<'a> CustomSlider<'a> {
    fn new(param: &'a mut SliderParam) -> Self {
        Self { param }
    }
}

impl<'a> Widget for CustomSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let param = self.param;
        let _id = ui.make_persistent_id(&param.name);

        ui.vertical(|ui| {
            ui.label(format!("{}: {}", &param.name, param.value));

            let desired_size = egui::vec2(ui.available_width(), 20.0);
            let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::drag());

            // スライダの「棒」の色をテーマに合わせる
            let line_color = ui.visuals().widgets.inactive.bg_fill; // 非アクティブ時の背景色を棒の色に
            let line_y = rect.center().y;
            let line_start = Pos2::new(rect.left(), line_y);
            let line_end = Pos2::new(rect.right(), line_y);
            let line_stroke = Stroke::new(2.0, line_color);
            ui.painter().line_segment([line_start, line_end], line_stroke);

            // スライダの「ハンドル」の色をテーマに合わせる
            let handle_color = if response.dragged() {
                ui.visuals().widgets.active.bg_fill
            } else {
                ui.visuals().widgets.inactive.bg_fill
            };

            let normalized = (param.value - param.min) / (param.max - param.min);
            let handle_pos = Pos2::new(
                rect.left() + (normalized * rect.width() as f64) as f32,
                line_y,
            );

            // ハンドルのボーダーを描画
            let border_radius = 10.5; // ボーダーを含む外側の円の半径
            let border_color = if response.dragged() || response.hovered() {
                ui.visuals().widgets.active.fg_stroke.color
            } else {
                ui.visuals().widgets.inactive.fg_stroke.color
            };
            ui.painter().circle_filled(handle_pos, border_radius, border_color);

            // ハンドルの本体（内側）を描画
            let handle_radius = 10.0; // ハンドル本体の半径
            ui.painter().circle_filled(handle_pos, handle_radius, handle_color); // ハンドルの色を適用

            if response.dragged() {
                let delta_x = ui.input(|i| i.pointer.delta().x) as f64;
                let range = param.max - param.min;
                
                let value_delta = (delta_x / rect.width() as f64) * range;
                
                let mut new_value = param.value + value_delta;

                new_value = new_value.clamp(param.min, param.max);

                if param.step > 0.0 {
                    new_value = (new_value / param.step).round() * param.step;
                    new_value = new_value.clamp(param.min, param.max);
                }
                
                if (param.value - new_value).abs() > f64::EPSILON * new_value.abs().max(1.0) {
                    param.value = new_value;
                    response.mark_changed();
                }
            }

            response
        })
        .inner
    }
}