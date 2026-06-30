use aviutl2::{AnyResult, tracing};

#[aviutl2::plugin(GenericPlugin)]
struct WaveformPreviewPlugin {}

impl aviutl2::generic::GenericPlugin for WaveformPreviewPlugin {
    fn new(_info: aviutl2::common::AviUtl2Info) -> AnyResult<Self> {
        Self::init_logging();
        Ok(Self {})
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

    fn register(&mut self, _registry: &mut aviutl2::generic::HostAppHandle) {}
}

impl WaveformPreviewPlugin {
    fn init_logging() {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
    }
}

aviutl2::register_generic_plugin!(WaveformPreviewPlugin);
