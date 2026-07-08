use std::sync::mpsc;
use std::sync::{LazyLock, Mutex};
use std::thread;

use aviutl2::{AnyResult, anyhow, tracing};

use crate::EDIT_HANDLE;
use crate::config::{AnalysisAccuracy, AnalysisConfig, AnalysisRange};
use crate::utils::NChunks;

#[derive(Debug, Default, Clone)]
pub struct StereoWaveformBin {
    pub left: WaveformBin,
    pub right: WaveformBin,
}

impl StereoWaveformBin {
    fn from_samples(left: &[f32], right: &[f32]) -> Self {
        Self {
            left: WaveformBin::from_samples(left),
            right: WaveformBin::from_samples(right),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct WaveformBin {
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

impl WaveformBin {
    fn from_samples(samples: &[f32]) -> Self {
        match samples.split_first() {
            Some((&first, rest)) => {
                let mut min = first;
                let mut max = first;
                let first64 = first as f64;
                let mut rms = first64 * first64;

                for &value in rest {
                    min = min.min(value);
                    max = max.max(value);
                    let value64 = value as f64;
                    rms += value64 * value64;
                }

                let rms = (rms / samples.len() as f64).sqrt() as f32;

                Self { min, max, rms }
            }
            None => Self::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum WaveformAnalyzerStatus {
    Init,
    Done,
    Analyzing {
        completed_frame: u32,
        total_frame: u32,
    },
    Canceled,
    Failed {
        message: String,
    },
}

impl WaveformAnalyzerStatus {
    pub fn is_analyzing(&self) -> bool {
        matches!(self, Self::Analyzing { .. })
    }
}

pub struct WaveformReport {
    pub params: AnalysisParams,
    pub bins: Vec<StereoWaveformBin>,
}
pub static WAVEFORM_REPORT: Mutex<WaveformReport> = Mutex::new(WaveformReport {
    params: AnalysisParams {
        start: 0,
        end: 0,
        accuracy: AnalysisAccuracy::Medium,
        fps: 30.0,
    },
    bins: vec![],
});

#[derive(Debug, Clone)]
pub struct AnalysisParams {
    pub start: u32,
    pub end: u32,
    pub accuracy: AnalysisAccuracy,
    pub fps: f64,
}

#[derive(Debug)]
enum AnalyzeOutcome {
    Done,
    Restart(AnalysisParams),
    Canceled,
    Shutdown,
}

#[derive(Debug, Clone)]
enum AnalyzerCommand {
    Analyze(AnalysisParams),
    Cancel,
    Shutdown,
}

struct WaveformAnalyzer {
    worker: Option<thread::JoinHandle<()>>,
    tx: Option<mpsc::Sender<AnalyzerCommand>>,
    status: WaveformAnalyzerStatus,
}
static WAVEFORM_ANALYZER: LazyLock<Mutex<WaveformAnalyzer>> =
    LazyLock::new(|| Mutex::new(WaveformAnalyzer::new()));

impl WaveformAnalyzer {
    fn new() -> Self {
        Self {
            worker: None,
            tx: None,
            status: WaveformAnalyzerStatus::Init,
        }
    }

    fn is_running(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
    }

    fn start(&mut self) {
        if self.is_running() {
            return;
        }

        if let Some(worker) = self.worker.take()
            && let Err(err) = worker.join()
        {
            tracing::error!("Waveform Worker: join failed: {:?}", err);
        }
        self.tx = None;

        tracing::debug!("Start Analyze worker");

        let (tx, rx) = mpsc::channel::<AnalyzerCommand>();
        self.tx = Some(tx);
        self.worker = Some(thread::spawn(move || {
            Self::worker_main(rx);
        }));
    }

    fn worker_main(command_rx: mpsc::Receiver<AnalyzerCommand>) {
        tracing::debug!("Waveform Worker: Start");

        'main: while let Ok(command) = command_rx.recv() {
            match command {
                AnalyzerCommand::Analyze(mut params) => {
                    tracing::debug!("Waveform Worker: Analyze command");
                    'analyze: loop {
                        match Self::worker_analyze(&command_rx, params) {
                            Ok(AnalyzeOutcome::Done) => {
                                break 'analyze;
                            }
                            Ok(AnalyzeOutcome::Restart(new_params)) => {
                                params = new_params;
                            }
                            Ok(AnalyzeOutcome::Canceled) => {
                                tracing::info!("Analyze canceled");
                                break 'analyze;
                            }
                            Ok(AnalyzeOutcome::Shutdown) => break 'main,
                            Err(err) => {
                                tracing::error!("{}", err);
                                {
                                    let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
                                    analyzer.status = WaveformAnalyzerStatus::Failed {
                                        message: err.to_string(),
                                    };
                                }
                                break 'main;
                            }
                        }
                    }
                }
                AnalyzerCommand::Cancel => {
                    tracing::debug!("Waveform Worker: Cancel command");
                }
                AnalyzerCommand::Shutdown => {
                    tracing::debug!("Waveform Worker: Shutdown command");
                    break 'main;
                }
            }
            crate::request_repaint();
        }

        tracing::debug!("Waveform Worker: End");
    }

    fn worker_analyze(
        command_rx: &mpsc::Receiver<AnalyzerCommand>,
        params: AnalysisParams,
    ) -> AnyResult<AnalyzeOutcome> {
        tracing::debug!("Start analyzing: {:?}", params);
        let total_frame = params.end - params.start + 1;
        {
            let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
            analyzer.status = WaveformAnalyzerStatus::Analyzing {
                completed_frame: 0,
                total_frame,
            };
        }
        {
            let mut report = WAVEFORM_REPORT.lock().unwrap();
            report.params = params.clone();
            let len = ((params.end - params.start + 1) as usize) * params.accuracy.points();
            report.bins.clear();
            report.bins.reserve(len);
        }

        for i in params.start..=params.end {
            if let Some(outcome) = Self::receive_interrupt_command(command_rx)? {
                match outcome {
                    AnalyzeOutcome::Restart(params) => {
                        return Ok(AnalyzeOutcome::Restart(params));
                    }
                    AnalyzeOutcome::Canceled => {
                        {
                            let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
                            analyzer.status = WaveformAnalyzerStatus::Canceled;
                        }
                        return Ok(AnalyzeOutcome::Canceled);
                    }
                    AnalyzeOutcome::Shutdown => {
                        return Ok(AnalyzeOutcome::Shutdown);
                    }
                    AnalyzeOutcome::Done => unreachable!(),
                }
            }

            let result = EDIT_HANDLE.rendering_scene_audio(i, move |result| {
                let bins = Self::analyze_frame(result, params.accuracy);

                {
                    let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
                    analyzer.status = WaveformAnalyzerStatus::Analyzing {
                        completed_frame: i - params.start + 1,
                        total_frame,
                    };
                }

                let mut report = WAVEFORM_REPORT.lock().unwrap();
                report.bins.extend(bins);
            });
            if let Err(e) = result {
                let message = format!("Failed to rendering_scene_audio: {}", e);
                {
                    let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
                    analyzer.status = WaveformAnalyzerStatus::Failed {
                        message: message.clone(),
                    };
                }
                return Err(anyhow::anyhow!(message));
            }
            EDIT_HANDLE.wait_rendering_task();
        }

        {
            let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
            analyzer.status = WaveformAnalyzerStatus::Done;
        }

        Ok(AnalyzeOutcome::Done)
    }

    fn analyze_frame(
        frame: aviutl2::generic::RenderingSceneAudio,
        accuracy: AnalysisAccuracy,
    ) -> Vec<StereoWaveformBin> {
        let points = accuracy.points();

        let left_chunks = NChunks::new(frame.buffer0, points);
        let right_chunks = NChunks::new(frame.buffer1, points);

        left_chunks
            .zip(right_chunks)
            .map(|(left, right)| StereoWaveformBin::from_samples(left, right))
            .collect()
    }

    fn receive_interrupt_command(
        command_rx: &mpsc::Receiver<AnalyzerCommand>,
    ) -> AnyResult<Option<AnalyzeOutcome>> {
        let mut latest_analyze = None;

        loop {
            match command_rx.try_recv() {
                Ok(AnalyzerCommand::Analyze(params)) => {
                    latest_analyze = Some(params);
                }
                Ok(AnalyzerCommand::Cancel) => {
                    return Ok(Some(AnalyzeOutcome::Canceled));
                }
                Ok(AnalyzerCommand::Shutdown) => {
                    return Ok(Some(AnalyzeOutcome::Shutdown));
                }
                Err(mpsc::TryRecvError::Empty) => {
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(anyhow::anyhow!("Channel is disconnected"));
                }
            }
        }

        if let Some(params) = latest_analyze {
            Ok(Some(AnalyzeOutcome::Restart(params)))
        } else {
            Ok(None)
        }
    }
}

pub fn get_status() -> WaveformAnalyzerStatus {
    WAVEFORM_ANALYZER.lock().unwrap().status.clone()
}

pub fn analyze(config: &AnalysisConfig) {
    let edit_info = EDIT_HANDLE.get_edit_info();

    let params = {
        let (start, end) = match config.range {
            AnalysisRange::All => (0, edit_info.frame_max as u32),
            AnalysisRange::Selected => {
                if let Some((start, end)) =
                    edit_info.select_range_start.zip(edit_info.select_range_end)
                {
                    (start as u32, end as u32)
                } else {
                    (0, edit_info.frame_max as u32)
                }
            }
            AnalysisRange::VisibleTimeline => (
                edit_info.display_frame_start as u32,
                (edit_info.display_frame_start + edit_info.display_frame_num - 1)
                    .min(edit_info.frame_max) as u32,
            ),
        };
        let fps = *edit_info.fps.numer() as f64 / *edit_info.fps.denom() as f64;

        AnalysisParams {
            start,
            end,
            accuracy: config.accuracy,
            fps,
        }
    };

    let tx = {
        let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
        analyzer.start();
        analyzer.tx.clone()
    };

    if let Some(tx) = tx
        && let Err(err) = tx.send(AnalyzerCommand::Analyze(params))
    {
        tracing::error!("Waveform Worker: failed to send Analyze command: {}", err);
    }
}

pub fn cancel() {
    let tx = {
        let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
        if !analyzer.is_running() || !analyzer.status.is_analyzing() {
            return;
        }
        analyzer.start();
        analyzer.tx.clone()
    };

    if let Some(tx) = tx
        && let Err(err) = tx.send(AnalyzerCommand::Cancel)
    {
        tracing::error!("Waveform Worker: failed to send Cancel command: {}", err);
    }
}

pub fn shutdown() {
    let (tx, worker) = {
        let mut analyzer = WAVEFORM_ANALYZER.lock().unwrap();
        let tx = analyzer.tx.take();
        let worker = analyzer.worker.take();
        (tx, worker)
    };

    if let Some(tx) = tx {
        if let Err(err) = tx.send(AnalyzerCommand::Shutdown) {
            tracing::warn!("Waveform Worker: failed to send Shutdown command: {}", err);
        }

        drop(tx);
    }

    if let Some(worker) = worker
        && let Err(err) = worker.join()
    {
        tracing::error!("Waveform Worker: join failed: {:?}", err);
    }
}
