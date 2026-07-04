mod analyzer;
mod config;
mod gui;
mod utils;

use aviutl2::{AnyResult, generic::GenericPlugin, tracing};

use crate::config::ANALYSIS_CONFIG;

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

        registry.register_edit_menu("波形プレビュー\\解析開始", || {
            let config = ANALYSIS_CONFIG.lock().unwrap().analysis.clone();
            tracing::info!(
                "Edit Menu: 解析開始 ({}, {}, {})",
                config.range,
                config.accuracy,
                config.immediate
            );
            crate::analyzer::analyze(&config);
        });
        registry.register_edit_menu("波形プレビュー\\キャンセル", || {
            tracing::info!("Edit Menu: 解析キャンセル");
            crate::analyzer::cancel();
        });

        registry.register_event_listener(aviutl2::generic::EventType::ChangeEditFrame, || {
            tracing::info!("Change Edit Frame");
            request_repaint();
        });
        registry.register_event_listener(aviutl2::generic::EventType::ChangeEditScene, || {
            tracing::info!("Change Edit Scene");
            let config = ANALYSIS_CONFIG.lock().unwrap().analysis.clone();
            if config.immediate {
                crate::analyzer::analyze(&config);
            }
        });
        registry.register_event_listener(aviutl2::generic::EventType::UpdateObject, || {
            tracing::info!("Update Object");
            let config = ANALYSIS_CONFIG.lock().unwrap().analysis.clone();
            if config.immediate {
                crate::analyzer::analyze(&config);
            }
        });
    }
}

impl Drop for WaveformPreviewPlugin {
    fn drop(&mut self) {
        tracing::info!("WaveformPreview Dropped !");
        {
            let config = ANALYSIS_CONFIG.lock().unwrap();
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

aviutl2::register_generic_plugin!(WaveformPreviewPlugin);
