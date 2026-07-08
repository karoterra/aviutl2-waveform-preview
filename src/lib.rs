mod analyzer;
mod bpm;
mod config;
mod gui;
mod utils;

use aviutl2::{AnyResult, generic::GenericPlugin, tracing};

use crate::config::PLUGIN_CONFIG;

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
struct WaveformPreviewPlugin {
    window: aviutl2_eframe::EframeWindow,
}

impl aviutl2::generic::GenericPlugin for WaveformPreviewPlugin {
    fn new(_info: aviutl2::common::AviUtl2Info) -> AnyResult<Self> {
        Self::init_logging();
        let window = aviutl2_eframe::EframeWindow::new("WaveformPreview", |cc, handle| {
            Ok(Box::new(gui::WaveformPreviewApp::new(cc, handle)))
        })?;
        Ok(Self { window })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "WaveformPreview".to_string(),
            information: format!(
                "WaveformPreview v{} by karoterra",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        EDIT_HANDLE.init(registry.create_edit_handle());

        if let Ok(handle) = self.window.handle() {
            registry
                .register_window_client("波形プレビュー", &handle)
                .unwrap();
        }
    }

    fn event_change_edit_frame(&mut self) {
        tracing::debug!("Change Edit Frame");
        self.request_repaint();
    }

    fn event_change_scene_info(&mut self) {
        tracing::debug!("Change Scene Info");
        match EDIT_HANDLE.call_read_section(|read_section| read_section.get_grid_bpm_list()) {
            Ok(Ok(bpm_list)) => {
                crate::bpm::set_bpm_list(&bpm_list);
            }
            Ok(Err(err)) => {
                tracing::error!("Failed to get bpm list: {}", err);
            }
            Err(err) => {
                tracing::error!("Failed to get bpm list: {}", err);
            }
        }
        let config = PLUGIN_CONFIG.lock().unwrap().analysis.clone();
        if config.immediate {
            crate::analyzer::analyze(&config);
        }
    }

    fn event_update_object_info(&mut self) {
        tracing::debug!("Update Object");
        let config = PLUGIN_CONFIG.lock().unwrap().analysis.clone();
        if config.immediate {
            crate::analyzer::analyze(&config);
        }
    }
}

#[aviutl2::generic::menus]
impl WaveformPreviewPlugin {
    #[edit(name = "波形プレビュー\\解析開始")]
    fn analyze_waveform() -> AnyResult<()> {
        let config = PLUGIN_CONFIG.lock().unwrap().analysis.clone();
        tracing::debug!(
            "Edit Menu: 解析開始 ({}, {}, {})",
            config.range,
            config.accuracy,
            config.immediate
        );
        crate::analyzer::analyze(&config);
        Ok(())
    }

    #[edit(name = "波形プレビュー\\キャンセル")]
    fn cancel_analysis() -> AnyResult<()> {
        tracing::debug!("Edit Menu: 解析キャンセル");
        crate::analyzer::cancel();
        Ok(())
    }
}

impl Drop for WaveformPreviewPlugin {
    fn drop(&mut self) {
        tracing::debug!("WaveformPreview Dropped !");
        {
            let config = PLUGIN_CONFIG.lock().unwrap();
            if let Err(err) = config.save() {
                tracing::error!("Failed to save config: {}", err);
            }
        }
        crate::analyzer::shutdown();
    }
}

impl WaveformPreviewPlugin {
    fn init_logging() {
        if let Err(err) = aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .try_init()
        {
            tracing::error!("Failed to init logger: {}", err);
        }
    }

    fn request_repaint(&self) {
        match self.window.egui_ctx() {
            Ok(ctx) => {
                ctx.request_repaint();
            }
            Err(err) => {
                tracing::error!("Failed to get egui context: {}", err);
            }
        };
    }
}

pub fn request_repaint() {
    WaveformPreviewPlugin::with_instance(|inst| {
        inst.request_repaint();
    });
}

pub fn set_frame(frame: usize) {
    let layer = EDIT_HANDLE.get_edit_info().layer;
    let result = EDIT_HANDLE.call_edit_section(|edit| edit.set_cursor_layer_frame(layer, frame));
    match result {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            tracing::error!("Failed to set frame: {}", err);
        }
        Err(err) => {
            tracing::error!("Failed to set frame: {}", err);
        }
    }
}

aviutl2::register_generic_plugin!(WaveformPreviewPlugin);
