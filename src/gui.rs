use aviutl2::tracing;
use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use egui_plot::{AxisHints, FilledArea, GridMark, Plot, VLine};

use crate::EDIT_HANDLE;
use crate::analyzer::{StereoWaveformBin, WAVEFORM_REPORT, WaveformAnalyzerStatus, WaveformReport};
use crate::config::{ANALYSIS_CONFIG, AnalysisAccuracy, AnalysisRange, PluginConfig, ViewConfig};

pub struct WaveformPreviewApp {
    config_panel: bool,
}

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

        Self {
            config_panel: false,
        }
    }

    fn waveform_area(
        &self,
        xs: &[f64],
        bins: &[StereoWaveformBin],
        config: &ViewConfig,
    ) -> (FilledArea, FilledArea) {
        let left_min: Vec<f64> = bins.iter().map(|bin| bin.left.min as f64).collect();
        let left_max: Vec<f64> = bins.iter().map(|bin| bin.left.max as f64).collect();
        let right_min: Vec<f64> = bins.iter().map(|bin| bin.right.min as f64).collect();
        let right_max: Vec<f64> = bins.iter().map(|bin| bin.right.max as f64).collect();

        let left =
            FilledArea::new("left", &xs, &left_min, &left_max).fill_color(config.waveform_color);
        let right =
            FilledArea::new("right", &xs, &right_min, &right_max).fill_color(config.waveform_color);

        (left, right)
    }

    fn rms_area(
        &self,
        xs: &[f64],
        bins: &[StereoWaveformBin],
        config: &ViewConfig,
    ) -> (FilledArea, FilledArea) {
        let left_min: Vec<f64> = bins.iter().map(|bin| -bin.left.rms as f64).collect();
        let left_max: Vec<f64> = bins.iter().map(|bin| bin.left.rms as f64).collect();
        let right_min: Vec<f64> = bins.iter().map(|bin| -bin.right.rms as f64).collect();
        let right_max: Vec<f64> = bins.iter().map(|bin| bin.right.rms as f64).collect();

        let left =
            FilledArea::new("rms_left", &xs, &left_min, &left_max).fill_color(config.rms_color);
        let right =
            FilledArea::new("rms_right", &xs, &right_min, &right_max).fill_color(config.rms_color);

        (left, right)
    }

    fn show_plot(&self, ui: &mut egui::Ui, report: &WaveformReport, config: &ViewConfig) {
        let edit_info = EDIT_HANDLE.get_edit_info();

        let points_per_frame = report.params.accuracy.points();
        let fps = report.params.fps;

        let start_sec = report.params.start as f64 / fps;
        let xs: Vec<f64> = (0..report.bins.len())
            .map(|i| start_sec + i as f64 / points_per_frame as f64 / fps)
            .collect();

        let (area_left, area_right) = self.waveform_area(&xs, &report.bins, &config);
        let (rms_left, rms_right) = self.rms_area(&xs, &report.bins, &config);

        let cursor = VLine::new("WaveformPlot_cursor", edit_info.frame as f64 / fps)
            .color(config.frame_cursor_color);

        let selected_span = edit_info
            .select_range_start
            .zip(edit_info.select_range_end)
            .map(|(start, end)| {
                let start = start as f64 / fps;
                let end = end as f64 / fps;
                egui_plot::Span::new("", start..=end).fill(config.selected_span_color)
            });

        let out_of_scene_span_left =
            egui_plot::Span::new("", f64::NEG_INFINITY..=0.0).fill(config.out_of_scene_span_color);
        let out_of_scene_span_right =
            egui_plot::Span::new("", (edit_info.frame_max as f64 / fps)..=f64::INFINITY)
                .fill(config.out_of_scene_span_color);

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

    fn show_config(&mut self, ui: &mut egui::Ui, config: &mut PluginConfig) {
        ui.heading("解析");
        ui.separator();
        egui::Grid::new("analysis_config_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("即時解析");
                ui.checkbox(&mut config.analysis.immediate, "オン");
                ui.end_row();

                ui.label("解析対象");
                egui::ComboBox::from_id_salt("解析対象")
                    .selected_text(config.analysis.range.to_string())
                    .show_ui(ui, |ui| {
                        for x in [AnalysisRange::All, AnalysisRange::Selected] {
                            ui.selectable_value(&mut config.analysis.range, x, x.to_string());
                        }
                    });
                ui.end_row();

                ui.label("解析精度");
                egui::ComboBox::from_id_salt("解析精度")
                    .selected_text(config.analysis.accuracy.to_string())
                    .show_ui(ui, |ui| {
                        for x in [
                            AnalysisAccuracy::Low,
                            AnalysisAccuracy::Medium,
                            AnalysisAccuracy::High,
                            AnalysisAccuracy::VeryHigh,
                        ] {
                            ui.selectable_value(&mut config.analysis.accuracy, x, x.to_string());
                        }
                    });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.heading("表示");
        ui.separator();
        egui::Grid::new("view_config_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("波形色");
                ui.color_edit_button_srgba(&mut config.view.waveform_color);
                ui.end_row();

                ui.label("RMS色");
                ui.color_edit_button_srgba(&mut config.view.rms_color);
                ui.end_row();

                ui.label("カーソルの色");
                ui.color_edit_button_srgba(&mut config.view.frame_cursor_color);
                ui.end_row();

                ui.label("選択範囲の色");
                ui.color_edit_button_srgba(&mut config.view.selected_span_color);
                ui.end_row();

                ui.label("シーン範囲外の色");
                ui.color_edit_button_srgba(&mut config.view.out_of_scene_span_color);
                ui.end_row();
            });
    }
}

impl eframe::App for WaveformPreviewApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut config = ANALYSIS_CONFIG.lock().unwrap();
        let status = crate::analyzer::get_status();

        egui::Panel::top("toolbar_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if status.is_analyzing() {
                    if ui.button("キャンセル").clicked() {
                        tracing::info!("キャンセル");
                        crate::analyzer::cancel();
                    }
                } else {
                    if ui.button("解析開始").clicked() {
                        tracing::info!("解析開始");
                        crate::analyzer::analyze(&config.analysis);
                    }
                }

                if ui
                    .checkbox(&mut config.analysis.immediate, "即時")
                    .changed()
                {
                    tracing::info!("即時モード: {}", config.analysis.immediate);
                }

                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    if ui.button("設定").clicked() {
                        self.config_panel = !self.config_panel;
                    }
                });
            });
        });

        egui::Panel::bottom("status_panel").show_inside(ui, |ui| {
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
        egui::Panel::right("config_panel").show_animated_inside(ui, self.config_panel, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.show_config(ui, &mut config);
            });
        });

        match status {
            WaveformAnalyzerStatus::Done => {
                let report = WAVEFORM_REPORT.lock().unwrap();
                self.show_plot(ui, &report, &config.view);
            }
            _ => {}
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.window_fill.to_normalized_gamma_f32()
    }
}
