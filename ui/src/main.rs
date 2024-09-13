use tracing::subscriber::set_global_default;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

fn main() {
    // dioxus_logger::init(dioxus_logger::tracing::Level::DEBUG).expect("logger failed to init");
    // dioxus v0.5.6 dumps info logs on amount of listeners in virtual_dom, which just flood console
    let layer_config = tracing_wasm::WASMLayerConfigBuilder::new()
        .set_max_level(tracing::Level::DEBUG)
        .build();
    let filter = EnvFilter::builder()
        .parse("dioxus_core=error,dioxus_web=error,dioxus_signals=error,info")
        .expect("failed to parse filter directive");
    let layer = tracing_wasm::WASMLayer::new(layer_config);
    let reg = Registry::default().with(layer).with(filter);
    set_global_default(reg).expect("failed to init tracing");
    dioxus::launch(ui::components::app::App)
}
