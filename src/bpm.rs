use std::{ops::RangeInclusive, sync::Mutex};

use aviutl2::generic::BpmInfo;

#[derive(Debug, Clone)]
pub struct BpmPlotInfo {
    /// BPM表示範囲の始まり(秒)
    pub min: f64,
    /// BPM表示範囲の終わり(秒)
    pub max: f64,
    /// テンポ
    pub tempo: f64,
    /// 拍子
    pub beat: i32,
    /// BPM開始位置(秒)
    pub start: f64,
    /// 拍子オフセット(秒)
    pub offset: f64,
}
static BPM_LIST: Mutex<Vec<BpmPlotInfo>> = Mutex::new(vec![]);

impl BpmPlotInfo {
    pub fn range(&self) -> RangeInclusive<f64> {
        self.min..=self.max
    }
}

pub fn set_bpm_list(raw_list: &Vec<BpmInfo>) {
    let mut bpm_list = BPM_LIST.lock().unwrap();
    bpm_list.clear();
    bpm_list.extend(get_bpm_list_from_raw(raw_list));
}

pub fn get_bpm_list() -> Vec<BpmPlotInfo> {
    BPM_LIST.lock().unwrap().clone()
}

fn get_bpm_list_from_raw(raw_list: &Vec<BpmInfo>) -> Vec<BpmPlotInfo> {
    let mut raw_list = raw_list.clone();
    raw_list.sort_unstable_by(|a, b| a.start.total_cmp(&b.start));

    let mut result: Vec<BpmPlotInfo> = raw_list
        .iter()
        .map(|x| BpmPlotInfo {
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            tempo: x.tempo as f64,
            beat: x.beat,
            start: x.start,
            offset: x.offset as f64,
        })
        .collect();

    let len = result.len();
    for i in 0..len {
        if i > 0 {
            result[i].min = result[i].start;
        }
        if i < len.saturating_sub(1) {
            result[i].max = result[i + 1].start;
        }
    }

    result
}
