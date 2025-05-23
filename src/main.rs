use eframe::{egui, App, Frame};
use egui_plot::{Line, Plot, PlotPoints};

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
            num_points: 500, // デフォルトで500点
        }
    }
}

impl App for ParametricPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ウインドウサイズを取得
            let window_height = ctx.input(|input| input.screen_rect().height());
            let window_width = ctx.input(|input| input.screen_rect().width());
            let plot_size = egui::vec2(window_width, window_height);
            // グラフを画面いっぱいに表示（幅＝ウインドウ幅、高さ＝ウインドウ高さ）
            let mut points: Vec<[f64; 2]> = Vec::with_capacity(self.num_points);
            let t_range = self.t_max - self.t_min;
            for i in 0..self.num_points {
                let t = self.t_min + (i as f64 / (self.num_points - 1) as f64) * t_range;
                let x = self.a * t.cos();
                let y = self.b * t.sin();
                points.push([x, y]);
            }
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
