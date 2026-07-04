use std::sync::{LazyLock, Mutex};

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub range: AnalysisRange,
    pub accuracy: AnalysisAccuracy,
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

pub static ANALYSIS_CONFIG: LazyLock<Mutex<AnalysisConfig>> =
    LazyLock::new(|| Mutex::new(AnalysisConfig::default()));
