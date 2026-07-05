use std::{
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use aviutl2::{AnyResult, anyhow, tracing};
use aviutl2_eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum AnalysisRange {
    All,
    Selected,
}

impl Default for AnalysisRange {
    fn default() -> Self {
        Self::All
    }
}

impl std::fmt::Display for AnalysisRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisRange::All => write!(f, "全体"),
            AnalysisRange::Selected => write!(f, "選択範囲"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum AnalysisAccuracy {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl Default for AnalysisAccuracy {
    fn default() -> Self {
        Self::Medium
    }
}

impl AnalysisAccuracy {
    pub fn points(&self) -> usize {
        match self {
            AnalysisAccuracy::Low => 1,
            AnalysisAccuracy::Medium => 2,
            AnalysisAccuracy::High => 4,
            AnalysisAccuracy::VeryHigh => 8,
        }
    }
}

impl std::fmt::Display for AnalysisAccuracy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisAccuracy::Low => write!(f, "低"),
            AnalysisAccuracy::Medium => write!(f, "標準"),
            AnalysisAccuracy::High => write!(f, "高"),
            AnalysisAccuracy::VeryHigh => write!(f, "最高"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    #[serde(default)]
    pub range: AnalysisRange,

    #[serde(default)]
    pub accuracy: AnalysisAccuracy,

    #[serde(default)]
    pub immediate: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            range: AnalysisRange::default(),
            accuracy: AnalysisAccuracy::default(),
            immediate: false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ViewScaleY {
    Linear,
    Decibel,
}

impl Default for ViewScaleY {
    fn default() -> Self {
        ViewScaleY::Linear
    }
}

impl std::fmt::Display for ViewScaleY {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewScaleY::Linear => write!(f, "リニア"),
            ViewScaleY::Decibel => write!(f, "dB"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    #[serde(default)]
    pub scale_y: ViewScaleY,

    #[serde(default = "ViewConfig::default_waveform_color")]
    pub waveform_color: Color32,

    #[serde(default = "ViewConfig::default_rms_color")]
    pub rms_color: Color32,

    #[serde(default = "ViewConfig::default_frame_cursor_color")]
    pub frame_cursor_color: Color32,

    #[serde(default = "ViewConfig::default_selected_span_color")]
    pub selected_span_color: Color32,

    #[serde(default = "ViewConfig::default_out_of_scene_span_color")]
    pub out_of_scene_span_color: Color32,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            scale_y: ViewScaleY::default(),
            waveform_color: Self::default_waveform_color(),
            rms_color: Self::default_rms_color(),
            frame_cursor_color: Self::default_frame_cursor_color(),
            selected_span_color: Self::default_selected_span_color(),
            out_of_scene_span_color: Self::default_out_of_scene_span_color(),
        }
    }
}

impl ViewConfig {
    fn default_waveform_color() -> Color32 {
        Color32::from_rgba_unmultiplied(100, 200, 100, 255)
    }

    fn default_rms_color() -> Color32 {
        Color32::from_rgba_unmultiplied(50, 255, 50, 255)
    }

    fn default_frame_cursor_color() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 0, 0, 255)
    }

    fn default_selected_span_color() -> Color32 {
        Color32::from_rgba_unmultiplied(107, 195, 225, 70)
    }

    fn default_out_of_scene_span_color() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 60)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "PluginConfig::default_version")]
    pub version: u32,

    #[serde(default)]
    pub analysis: AnalysisConfig,

    #[serde(default)]
    pub view: ViewConfig,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            version: Self::default_version(),
            analysis: AnalysisConfig::default(),
            view: ViewConfig::default(),
        }
    }
}

impl PluginConfig {
    fn default_version() -> u32 {
        1
    }

    fn get_path() -> AnyResult<PathBuf> {
        let dll_path =
            process_path::get_dylib_path().ok_or(anyhow::anyhow!("Failed to get dll path"))?;
        let dll_dir = dll_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or(anyhow::anyhow!("Failed to resolve dll directory"))?;
        let config_path = dll_dir.join("waveformpreview_config.json");
        Ok(config_path)
    }

    pub fn load() -> AnyResult<Self> {
        let config_path = Self::get_path()?;
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(config_path)?;
        let config: Self = serde_json::from_str(&text)?;
        Ok(config)
    }

    pub fn save(&self) -> AnyResult<()> {
        let config_path = Self::get_path()?;
        let serialized = serde_json::to_string_pretty(&self)?;
        fs::write(config_path, serialized)?;
        Ok(())
    }
}

pub static ANALYSIS_CONFIG: LazyLock<Mutex<PluginConfig>> = LazyLock::new(|| {
    let config = match PluginConfig::load() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Failed to load config: {}", err);
            PluginConfig::default()
        }
    };
    Mutex::new(config)
});
