use aviutl2::tracing;
use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use egui_plot::{AxisHints, FilledArea, GridMark, Plot, VLine};

use crate::EDIT_HANDLE;
use crate::analyzer::{StereoWaveformBin, WAVEFORM_REPORT, WaveformAnalyzerStatus, WaveformReport};
use crate::config::{ANALYSIS_CONFIG, AnalysisAccuracy, AnalysisRange};

pub struct WaveformPreviewApp {}

fn time_formatter(mark: GridMark, _range: &std::ops::RangeInclusive<f64>) -> String {
    let total_centiseconds = (mark.value * 100.0).round() as u64;

    let hours = total_centiseconds / 360_000;
    let minutes = (total_centiseconds / 6_000) % 60;
    let seconds = (total_centiseconds / 100) % 60;
    let centiseconds = total_centiseconds % 100;

    format!(
        "{:02}:{:02}:{:02}.{:02}",
        hours, minutes, seconds, centiseconds
    )
}

impl WaveformPreviewApp {
    pub fn new(cc: &eframe::CreationContext<'_>, _handle: AviUtl2EframeHandle) -> Self {
        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = aviutl2_eframe::aviutl2_visuals();
        });
        cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());

        Self {}
    }

    fn waveform_area(&self, xs: &[f64], bins: &[StereoWaveformBin]) -> (FilledArea, FilledArea) {
        let color = egui::Color32::from_rgba_unmultiplied(100, 200, 100, 255);

        let left_min: Vec<f64> = bins.iter().map(|bin| bin.left.min as f64).collect();
        let left_max: Vec<f64> = bins.iter().map(|bin| bin.left.max as f64).collect();
        let right_min: Vec<f64> = bins.iter().map(|bin| bin.right.min as f64).collect();
        let right_max: Vec<f64> = bins.iter().map(|bin| bin.right.max as f64).collect();

        let left = FilledArea::new("left", &xs, &left_min, &left_max).fill_color(color);
        let right = FilledArea::new("right", &xs, &right_min, &right_max).fill_color(color);

        (left, right)
    }

    fn rms_area(&self, xs: &[f64], bins: &[StereoWaveformBin]) -> (FilledArea, FilledArea) {
        let color = egui::Color32::from_rgba_unmultiplied(50, 255, 50, 255);

        let left_min: Vec<f64> = bins.iter().map(|bin| -bin.left.rms as f64).collect();
        let left_max: Vec<f64> = bins.iter().map(|bin| bin.left.rms as f64).collect();
        let right_min: Vec<f64> = bins.iter().map(|bin| -bin.right.rms as f64).collect();
        let right_max: Vec<f64> = bins.iter().map(|bin| bin.right.rms as f64).collect();

        let left = FilledArea::new("rms_left", &xs, &left_min, &left_max).fill_color(color);
        let right = FilledArea::new("rms_right", &xs, &right_min, &right_max).fill_color(color);

        (left, right)
    }

    fn show_plot(&self, ui: &mut egui::Ui, report: &WaveformReport) {
        let edit_info = EDIT_HANDLE.get_edit_info();

        let points_per_frame = report.params.accuracy.points();
        let fps = report.params.fps;

        let start_sec = report.params.start as f64 / fps;
        let xs: Vec<f64> = (0..report.bins.len())
            .map(|i| start_sec + i as f64 / points_per_frame as f64 / fps)
            .collect();

        let (area_left, area_right) = self.waveform_area(&xs, &report.bins);
        let (rms_left, rms_right) = self.rms_area(&xs, &report.bins);

        let cursor = VLine::new("WaveformPlot_cursor", edit_info.frame as f64 / fps)
            .color(egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255));

        let selected_span = edit_info
            .select_range_start
            .zip(edit_info.select_range_end)
            .map(|(start, end)| {
                let start = start as f64 / fps;
                let end = end as f64 / fps;
                egui_plot::Span::new("", start..=end)
            });

        let out_of_scene_span_left = egui_plot::Span::new("", f64::NEG_INFINITY..=0.0)
            .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60));
        let out_of_scene_span_right =
            egui_plot::Span::new("", (edit_info.frame_max as f64 / fps)..=f64::INFINITY)
                .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60));

        let link_group_id = ui.id().with("WaveformPlot_LinkGroup");
        let link_vec = egui::Vec2b::new(true, true);

        let x_axes = vec![AxisHints::new_x().formatter(time_formatter)];

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let size = ui.available_size();
            let half_height = size.y / 2.0;

            ui.allocate_ui(egui::vec2(size.x, half_height), |ui| {
                Plot::new("WaveformPlot_Left")
                    .link_axis(link_group_id, link_vec)
                    .link_cursor(link_group_id, link_vec)
                    .default_y_bounds(-1.0, 1.0)
                    .center_y_axis(true)
                    .y_axis_label("L")
                    .custom_x_axes(x_axes.clone())
                    .allow_scroll(egui::Vec2b::new(true, false))
                    .allow_zoom(egui::Vec2b::new(true, false))
                    .show(ui, |plot_ui| {
                        plot_ui.span(out_of_scene_span_left.clone());
                        plot_ui.span(out_of_scene_span_right.clone());
                        if let Some(span) = selected_span.clone() {
                            plot_ui.span(span);
                        }
                        plot_ui.add(area_left);
                        plot_ui.add(rms_left);
                        plot_ui.vline(cursor.clone());
                    });
            });
            ui.allocate_ui(egui::vec2(size.x, half_height), |ui| {
                Plot::new("WaveformPlot_Right")
                    .link_axis(link_group_id, link_vec)
                    .link_cursor(link_group_id, link_vec)
                    .default_y_bounds(-1.0, 1.0)
                    .center_y_axis(true)
                    .y_axis_label("R")
                    .custom_x_axes(x_axes)
                    .allow_scroll(egui::Vec2b::new(true, false))
                    .allow_zoom(egui::Vec2b::new(true, false))
                    .show(ui, |plot_ui| {
                        plot_ui.span(out_of_scene_span_left);
                        plot_ui.span(out_of_scene_span_right);
                        if let Some(span) = selected_span {
                            plot_ui.span(span);
                        }
                        plot_ui.add(area_right);
                        plot_ui.add(rms_right);
                        plot_ui.vline(cursor);
                    });
            });
        });
    }
}

impl eframe::App for WaveformPreviewApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut config = ANALYSIS_CONFIG.lock().unwrap();
        let status = crate::analyzer::get_status();

        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if status.is_analyzing() {
                    if ui.button("キャンセル").clicked() {
                        tracing::info!("キャンセル");
                        crate::analyzer::cancel();
                    }
                } else {
                    if ui.button("解析開始").clicked() {
                        tracing::info!("解析開始");
                        crate::analyzer::analyze(&config);
                    }
                }

                if ui.checkbox(&mut config.immediate, "即時").changed() {
                    tracing::info!("即時モード: {}", config.immediate);
                }

                ui.label("解析対象");
                let before = config.range;
                egui::ComboBox::from_id_salt("解析対象")
                    .selected_text(config.range.to_string())
                    .show_ui(ui, |ui| {
                        for x in [AnalysisRange::All, AnalysisRange::Selected] {
                            ui.selectable_value(&mut config.range, x, x.to_string());
                        }
                    });
                if config.range != before {
                    tracing::info!("解析範囲: {} -> {}", before, config.range);
                }

                ui.label("解析精度");
                let before = config.accuracy;
                egui::ComboBox::from_id_salt("解析精度")
                    .selected_text(config.accuracy.to_string())
                    .show_ui(ui, |ui| {
                        for x in [
                            AnalysisAccuracy::Low,
                            AnalysisAccuracy::Medium,
                            AnalysisAccuracy::High,
                            AnalysisAccuracy::VeryHigh,
                        ] {
                            ui.selectable_value(&mut config.accuracy, x, x.to_string());
                        }
                    });
                if config.accuracy != before {
                    tracing::info!("解析精度: {} -> {}", before, config.accuracy);
                }
            });
        });

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| match status.clone() {
                WaveformAnalyzerStatus::Init => {}
                WaveformAnalyzerStatus::Done => {
                    ui.label("解析完了");
                }
                WaveformAnalyzerStatus::Analyzing {
                    completed_frame,
                    total_frame,
                } => {
                    ui.label("解析中");
                    let progress = if total_frame == 0 {
                        0.0
                    } else {
                        completed_frame as f32 / total_frame as f32
                    };
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .show_percentage()
                            .animate(true),
                    );
                }
                WaveformAnalyzerStatus::Canceled => {
                    ui.label("キャンセルされました");
                }
                WaveformAnalyzerStatus::Failed { message } => {
                    ui.label(format!("エラー: {}", message));
                }
            });
        });

        match status {
            WaveformAnalyzerStatus::Done => {
                let report = WAVEFORM_REPORT.lock().unwrap();
                self.show_plot(ui, &report);
            }
            _ => {}
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.window_fill.to_normalized_gamma_f32()
    }
}
