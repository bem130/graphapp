#![cfg(target_arch = "wasm32")]

use hframe::Aware;
use boa_engine::JsObject;
use eframe::{egui, Frame};
use egui_plot::{Line, Plot, PlotPoints};
use egui::Color32;
use boa_engine::{Context as BoaContext, Source, JsValue, JsArgs, NativeFunction, js_string, property::Attribute, property::PropertyKey};
use egui_commonmark;
use egui_extras::syntax_highlighting;
use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex};

static PENDING_CONTENT: Mutex<Option<String>> = Mutex::new(None);

const VIDEO: &str = r#"
"#;

#[derive(Default)]
pub struct MyApp {
    counter_open: bool,
    iframe_open: bool,
    yt_open: bool,
    count: i32,
    video_open: bool,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let style = egui::Style {
            visuals: egui::Visuals::dark(),
            ..Default::default()
        };

        cc.egui_ctx.set_style(style);

        Self {
            video_open: true,
            counter_open: true,
            iframe_open: true,
            yt_open: true,
            ..Default::default()
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        


        hframe::HtmlWindow::new("editor").content(VIDEO).show(ctx);
                egui::Window::new("None")
            .show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("Empty");
                });
            })
            .aware();

        egui::Window::new("Devtools")
            .show(ctx, |ui| {
                let video_toggle_text = if self.video_open {
                    "Force close video"
                } else {
                    "Open video"
                };
                if ui.button(video_toggle_text).clicked() {
                    self.video_open = !self.video_open;
                }
                ui.horizontal(|ui| {
                    ui.label("Counter controls: ");
                    if ui.button("+").clicked() {
                        self.count += 1;
                    }
                    if ui.button("-").clicked() {
                        self.count -= 1;
                    }
                });
                ui.horizontal(|ui| {
                    egui::warn_if_debug_build(ui);
                    egui::widgets::global_dark_light_mode_buttons(ui);
                });
            })
            .aware();
        
        hframe::sync(ctx);
        
    }
}


#[wasm_bindgen]
pub fn update(data: &str) {
    if let Ok(mut content) = PENDING_CONTENT.try_lock() {
        *content = Some(data.to_string());
    }
}