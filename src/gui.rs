use std::ops::RangeInclusive;

use aviutl2::config::translate;
use aviutl2::tracing;
use aviutl2_eframe::egui::{InnerResponse, Response};
use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use egui_plot::{FilledArea, GridMark, HLine, LineStyle, Plot, VLine};

use crate::EDIT_HANDLE;
use crate::analyzer::{StereoWaveformBin, WAVEFORM_REPORT, WaveformAnalyzerStatus, WaveformReport};
use crate::bpm::BpmPlotInfo;
use crate::config::{
    AnalysisAccuracy, AnalysisRange, PLUGIN_CONFIG, PluginConfig, ViewConfig, ViewScaleY,
};

pub struct WaveformPreviewApp {
    config_panel: bool,
    reset_plot: bool,
}

const MIN_DB: f64 = -60.0;

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

fn decibel_formatter(mark: GridMark, _range: &std::ops::RangeInclusive<f64>) -> String {
    let y = remap(mark.value.abs(), 0.0, 1.0, MIN_DB, 0.0);
    format!("{y:.0}")
}

fn linear(x: f32) -> f64 {
    x as f64
}

// x の値を [a, b] から [c, d] に線形写像する
fn remap(x: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    c + (x - a) * (d - c) / (b - a)
}

fn remap_decibel_to_linear(db: f64) -> f64 {
    remap(db, MIN_DB, 0.0, 0.0, 1.0)
}

fn decibel_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

fn decibel_bipolar(x: f32) -> f64 {
    x.signum() as f64 * decibel_unipolar(x)
}

fn decibel_unipolar(x: f32) -> f64 {
    let amp = (x as f64).abs();
    let db = if amp <= 0.0 {
        MIN_DB
    } else {
        20.0 * amp.log10()
    }
    .clamp(MIN_DB, 0.0);

    remap_decibel_to_linear(db)
}

impl WaveformPreviewApp {
    pub fn new(cc: &eframe::CreationContext<'_>, _handle: AviUtl2EframeHandle) -> Self {
        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = aviutl2_eframe::aviutl2_visuals();
        });
        cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());

        Self {
            config_panel: false,
            reset_plot: false,
        }
    }

    fn waveform_area(
        &self,
        xs: &[f64],
        bins: &[StereoWaveformBin],
        config: &ViewConfig,
    ) -> (FilledArea, FilledArea) {
        let (left_min, left_max, right_min, right_max): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) =
            match config.scale_y {
                ViewScaleY::Linear => (
                    bins.iter().map(|bin| linear(bin.left.min)).collect(),
                    bins.iter().map(|bin| linear(bin.left.max)).collect(),
                    bins.iter().map(|bin| linear(bin.right.min)).collect(),
                    bins.iter().map(|bin| linear(bin.right.max)).collect(),
                ),
                ViewScaleY::DecibelBipolar => (
                    bins.iter()
                        .map(|bin| decibel_bipolar(bin.left.min))
                        .collect(),
                    bins.iter()
                        .map(|bin| decibel_bipolar(bin.left.max))
                        .collect(),
                    bins.iter()
                        .map(|bin| decibel_bipolar(bin.right.min))
                        .collect(),
                    bins.iter()
                        .map(|bin| decibel_bipolar(bin.right.max))
                        .collect(),
                ),
                ViewScaleY::DecibelUnipolar => (
                    bins.iter().map(|_| 0.0).collect(),
                    bins.iter()
                        .map(|bin| decibel_unipolar(bin.left.max.abs().max(bin.left.min.abs())))
                        .collect(),
                    bins.iter().map(|_| 0.0).collect(),
                    bins.iter()
                        .map(|bin| decibel_unipolar(bin.right.max.abs().max(bin.right.min.abs())))
                        .collect(),
                ),
            };

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
        let (left_min, left_max, right_min, right_max): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) =
            match config.scale_y {
                ViewScaleY::Linear => (
                    bins.iter().map(|bin| -linear(bin.left.rms)).collect(),
                    bins.iter().map(|bin| linear(bin.left.rms)).collect(),
                    bins.iter().map(|bin| -linear(bin.right.rms)).collect(),
                    bins.iter().map(|bin| linear(bin.right.rms)).collect(),
                ),
                ViewScaleY::DecibelBipolar => (
                    bins.iter()
                        .map(|bin| -decibel_unipolar(bin.left.rms))
                        .collect(),
                    bins.iter()
                        .map(|bin| decibel_unipolar(bin.left.rms))
                        .collect(),
                    bins.iter()
                        .map(|bin| -decibel_unipolar(bin.right.rms))
                        .collect(),
                    bins.iter()
                        .map(|bin| decibel_unipolar(bin.right.rms))
                        .collect(),
                ),
                ViewScaleY::DecibelUnipolar => (
                    bins.iter().map(|_| 0.0).collect(),
                    bins.iter()
                        .map(|bin| decibel_unipolar(bin.left.rms))
                        .collect(),
                    bins.iter().map(|_| 0.0).collect(),
                    bins.iter()
                        .map(|bin| decibel_unipolar(bin.right.rms))
                        .collect(),
                ),
            };

        let left =
            FilledArea::new("rms_left", &xs, &left_min, &left_max).fill_color(config.rms_color);
        let right =
            FilledArea::new("rms_right", &xs, &right_min, &right_max).fill_color(config.rms_color);

        (left, right)
    }

    fn reference_line_value(config: &ViewConfig) -> f64 {
        let reference_db = config.reference_line_value_db.clamp(MIN_DB, 0.0);
        match config.scale_y {
            ViewScaleY::Linear => decibel_to_linear(reference_db),
            ViewScaleY::DecibelBipolar | ViewScaleY::DecibelUnipolar => {
                remap_decibel_to_linear(reference_db)
            }
        }
    }

    fn add_reference_line(plot_ui: &mut egui_plot::PlotUi<'_>, config: &ViewConfig) {
        if !config.reference_line_enabled {
            return;
        }

        let value = Self::reference_line_value(config);
        plot_ui.hline(
            HLine::new("WaveformPlot_reference_pos", value).color(config.reference_line_color),
        );

        if !matches!(config.scale_y, ViewScaleY::DecibelUnipolar) {
            plot_ui.hline(
                HLine::new("WaveformPlot_reference_neg", -value).color(config.reference_line_color),
            );
        }
    }

    fn add_bpm_grid(
        plot_ui: &mut egui_plot::PlotUi<'_>,
        config: &ViewConfig,
        bpm_list: &Vec<BpmPlotInfo>,
        analysis_range: &RangeInclusive<f64>,
    ) {
        if !config.bpm_grid_enabled {
            return;
        }

        let bounds = plot_ui.plot_bounds();
        let plot_range = crate::utils::intersection(&bounds.range_x(), analysis_range);
        if plot_range.is_none() {
            return;
        }
        let plot_range = plot_range.unwrap();
        let px = plot_ui.transform().frame().width() as f64;
        let sec = bounds.max()[0] - bounds.min()[0];
        if sec <= 0.0 || !sec.is_finite() {
            return;
        }
        let px_per_sec = px / sec;

        for bpm in bpm_list.iter() {
            if bpm.beat == 0 || bpm.tempo <= 0.0 {
                continue;
            }

            let range = crate::utils::intersection(&plot_range, &bpm.range());
            if range.is_none() {
                continue;
            }
            let range = range.unwrap();

            let beat_per_sec = bpm.tempo / 60.0;
            let px_per_beat = px_per_sec / beat_per_sec;
            let px_per_measure = px_per_beat * bpm.beat as f64;
            let offset = bpm.start + bpm.offset;
            let start_idx = ((range.start() - offset) * beat_per_sec).ceil() as i64;
            let end_idx = ((range.end() - offset) * beat_per_sec).floor() as i64;
            for i in start_idx..=end_idx {
                let sec = offset + i as f64 / beat_per_sec;
                let is_measure = i.rem_euclid(bpm.beat as i64) == 0;
                if !is_measure && px_per_beat >= 6.0 {
                    plot_ui.vline(
                        VLine::new("bpm_beat", sec)
                            .color(config.bpm_grid_beat_color)
                            .style(LineStyle::dashed_dense()),
                    );
                }
                if is_measure && px_per_measure >= 6.0 {
                    plot_ui.vline(
                        VLine::new("bpm_measure", sec)
                            .color(config.bpm_grid_measure_color)
                            .style(LineStyle::dashed_dense()),
                    );
                }
            }

            if range.contains(&bpm.start) {
                plot_ui.vline(
                    VLine::new("bpm_start", bpm.start)
                        .color(config.bpm_grid_start_color)
                        .width(3.0),
                );
            }
        }
    }

    fn new_plot(&mut self, ui: &mut egui::Ui, config: &ViewConfig) -> (Plot<'_>, Plot<'_>) {
        let link_group_id = ui.id().with("WaveformPlot_LinkGroup");
        let link_vec = egui::Vec2b::new(true, true);

        let left = Plot::new("WaveformPlot_Left")
            .link_axis(link_group_id, link_vec)
            .link_cursor(link_group_id, link_vec)
            .x_axis_formatter(time_formatter)
            .allow_drag(egui::Vec2b::new(true, false))
            .allow_axis_zoom_drag(egui::Vec2b::new(true, false))
            .allow_scroll(egui::Vec2b::new(true, false))
            .allow_zoom(egui::Vec2b::new(true, false));
        let right = Plot::new("WaveformPlot_Right")
            .link_axis(link_group_id, link_vec)
            .link_cursor(link_group_id, link_vec)
            .x_axis_formatter(time_formatter)
            .allow_drag(egui::Vec2b::new(true, false))
            .allow_axis_zoom_drag(egui::Vec2b::new(true, false))
            .allow_scroll(egui::Vec2b::new(true, false))
            .allow_zoom(egui::Vec2b::new(true, false));

        let (left, right) = match config.scale_y {
            ViewScaleY::Linear => (
                left.y_axis_label("L").default_y_bounds(-1.0, 1.0),
                right.y_axis_label("R").default_y_bounds(-1.0, 1.0),
            ),
            ViewScaleY::DecibelBipolar => (
                left.y_axis_label("L [dB]")
                    .y_axis_formatter(decibel_formatter)
                    .default_y_bounds(-1.0, 1.0),
                right
                    .y_axis_label("R [dB]")
                    .y_axis_formatter(decibel_formatter)
                    .default_y_bounds(-1.0, 1.0),
            ),
            ViewScaleY::DecibelUnipolar => (
                left.y_axis_label("L [dB]")
                    .y_axis_formatter(decibel_formatter)
                    .default_y_bounds(0.0, 1.0),
                right
                    .y_axis_label("R [dB]")
                    .y_axis_formatter(decibel_formatter)
                    .default_y_bounds(0.0, 1.0),
            ),
        };

        (left, right)
    }

    fn show_plot(&mut self, ui: &mut egui::Ui, report: &WaveformReport, config: &ViewConfig) {
        let edit_info = EDIT_HANDLE.get_edit_info();

        let points_per_frame = report.params.accuracy.points();
        let fps = report.params.fps;

        let start_sec = report.params.start as f64 / fps;
        let end_sec = report.params.end as f64 / fps;
        let analysis_range_sec = start_sec..=end_sec;
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

        let bpm_list = crate::bpm::get_bpm_list();

        let reset_plot = self.reset_plot;
        self.reset_plot = false;
        let range_y = match config.scale_y {
            ViewScaleY::Linear => -1.0..=1.0,
            ViewScaleY::DecibelBipolar => -1.0..=1.0,
            ViewScaleY::DecibelUnipolar => 0.0..=1.0,
        };

        let InnerResponse {
            inner:
                (
                    InnerResponse {
                        inner: left_response,
                        ..
                    },
                    InnerResponse {
                        inner: right_response,
                        ..
                    },
                ),
            ..
        } = egui::CentralPanel::default().show_inside(ui, |ui| {
            let size = ui.available_size();
            let half_height = size.y / 2.0;
            let (left_plot, right_plot) = self.new_plot(ui, &config);

            let left_response = ui.allocate_ui(egui::vec2(size.x, half_height), |ui| {
                left_plot.show(ui, |plot_ui| {
                    plot_ui.span(out_of_scene_span_left.clone());
                    plot_ui.span(out_of_scene_span_right.clone());
                    if let Some(span) = selected_span.clone() {
                        plot_ui.span(span);
                    }
                    plot_ui.add(area_left);
                    plot_ui.add(rms_left);
                    Self::add_bpm_grid(plot_ui, config, &bpm_list, &analysis_range_sec);
                    Self::add_reference_line(plot_ui, config);
                    plot_ui.vline(cursor.clone());

                    if reset_plot {
                        tracing::info!("Reset Y range");
                        plot_ui.set_plot_bounds_y(range_y.clone());
                    }

                    plot_ui.pointer_coordinate()
                })
            });
            let right_response = ui.allocate_ui(egui::vec2(size.x, half_height), |ui| {
                right_plot.show(ui, |plot_ui| {
                    plot_ui.span(out_of_scene_span_left);
                    plot_ui.span(out_of_scene_span_right);
                    if let Some(span) = selected_span {
                        plot_ui.span(span);
                    }
                    plot_ui.add(area_right);
                    plot_ui.add(rms_right);
                    Self::add_bpm_grid(plot_ui, config, &bpm_list, &analysis_range_sec);
                    Self::add_reference_line(plot_ui, config);
                    plot_ui.vline(cursor);

                    if reset_plot {
                        plot_ui.set_plot_bounds_y(range_y);
                    }

                    plot_ui.pointer_coordinate()
                })
            });

            (left_response, right_response)
        });

        if left_response.response.clicked()
            && let Some(pos) = left_response.inner
        {
            let new_frame = (pos.x * fps).floor() as usize;
            crate::set_frame(new_frame);
        }
        if right_response.response.clicked()
            && let Some(pos) = right_response.inner
        {
            let new_frame = (pos.x * fps).floor() as usize;
            crate::set_frame(new_frame);
        }
    }

    fn combobox<T: ToString + PartialEq + Copy>(
        &self,
        ui: &mut egui::Ui,
        id_salt: &str,
        selected: &mut T,
        values: &[T],
    ) -> Response {
        let response = egui::ComboBox::from_id_salt(id_salt)
            .selected_text(selected.to_string())
            .show_ui(ui, |ui| {
                for &x in values {
                    ui.selectable_value(selected, x, x.to_string());
                }
            });
        response.response
    }

    fn show_config(&mut self, ui: &mut egui::Ui, config: &mut PluginConfig) {
        ui.heading(translate("解析"));
        ui.separator();
        egui::Grid::new("analysis_config_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label(translate("即時解析"));
                ui.checkbox(&mut config.analysis.immediate, translate("オン"));
                ui.end_row();

                ui.label(translate("解析対象"));
                let values = [
                    AnalysisRange::All,
                    AnalysisRange::Selected,
                    AnalysisRange::VisibleTimeline,
                ];
                self.combobox(ui, "解析対象", &mut config.analysis.range, &values);
                ui.end_row();

                ui.label(translate("解析精度"));
                let values = [
                    AnalysisAccuracy::Low,
                    AnalysisAccuracy::Medium,
                    AnalysisAccuracy::High,
                    AnalysisAccuracy::VeryHigh,
                ];
                self.combobox(ui, "解析精度", &mut config.analysis.accuracy, &values);
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.heading(translate("表示"));
        ui.separator();
        egui::Grid::new("view_config_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label(translate("縦軸の単位"));
                let values = [
                    ViewScaleY::Linear,
                    ViewScaleY::DecibelBipolar,
                    ViewScaleY::DecibelUnipolar,
                ];
                let before = config.view.scale_y;
                self.combobox(ui, "縦軸の単位", &mut config.view.scale_y, &values);
                if config.view.scale_y != before {
                    self.reset_plot = true;
                }
                ui.end_row();

                ui.label(translate("波形色"));
                ui.color_edit_button_srgba(&mut config.view.waveform_color);
                ui.end_row();

                ui.label(translate("RMS色"));
                ui.color_edit_button_srgba(&mut config.view.rms_color);
                ui.end_row();

                ui.label(translate("カーソルの色"));
                ui.color_edit_button_srgba(&mut config.view.frame_cursor_color);
                ui.end_row();

                ui.label(translate("選択範囲の色"));
                ui.color_edit_button_srgba(&mut config.view.selected_span_color);
                ui.end_row();

                ui.label(translate("シーン範囲外の色"));
                ui.color_edit_button_srgba(&mut config.view.out_of_scene_span_color);
                ui.end_row();

                ui.label(translate("基準線を表示"));
                ui.checkbox(&mut config.view.reference_line_enabled, translate("オン"));
                ui.end_row();

                ui.label(translate("基準線の値"));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut config.view.reference_line_value_db)
                            .speed(0.1)
                            .range(MIN_DB..=0.0),
                    );
                    ui.label(format!(
                        "[dB] ≒ {:.3}",
                        decibel_to_linear(config.view.reference_line_value_db)
                    ));
                });
                ui.end_row();

                ui.label(translate("基準線の色"));
                ui.color_edit_button_srgba(&mut config.view.reference_line_color);
                ui.end_row();

                ui.label(translate("BPMグリッドを表示"));
                ui.checkbox(&mut config.view.bpm_grid_enabled, translate("オン"));
                ui.end_row();

                ui.label(translate("BPMグリッド(拍)の色"));
                ui.color_edit_button_srgba(&mut config.view.bpm_grid_beat_color);
                ui.end_row();

                ui.label(translate("BPMグリッド(小節)の色"));
                ui.color_edit_button_srgba(&mut config.view.bpm_grid_measure_color);
                ui.end_row();

                ui.label(translate("BPMグリッド(開始線)の色"));
                ui.color_edit_button_srgba(&mut config.view.bpm_grid_start_color);
                ui.end_row();
            });
    }
}

impl eframe::App for WaveformPreviewApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut config = PLUGIN_CONFIG.lock().unwrap();
        let status = crate::analyzer::get_status();

        egui::Panel::top("toolbar_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if status.is_analyzing() {
                    if ui.button(translate("キャンセル")).clicked() {
                        tracing::info!("キャンセル");
                        crate::analyzer::cancel();
                    }
                } else {
                    if ui.button(translate("解析開始")).clicked() {
                        tracing::info!("解析開始");
                        crate::analyzer::analyze(&config.analysis);
                    }
                }

                if ui
                    .checkbox(&mut config.analysis.immediate, translate("即時"))
                    .changed()
                {
                    tracing::info!("即時モード: {}", config.analysis.immediate);
                }

                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    if ui.button(translate("設定")).clicked() {
                        self.config_panel = !self.config_panel;
                    }
                });
            });
        });

        egui::Panel::bottom("status_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| match status.clone() {
                WaveformAnalyzerStatus::Init => {}
                WaveformAnalyzerStatus::Done => {
                    ui.label(translate("解析完了"));
                }
                WaveformAnalyzerStatus::Analyzing {
                    completed_frame,
                    total_frame,
                } => {
                    ui.label(translate("解析中"));
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
                    ui.label(translate("キャンセルされました"));
                }
                WaveformAnalyzerStatus::Failed { message } => {
                    ui.label(translate("エラー: {message}").replace("{message}", &message));
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
